//! All eight Class source domains retain their existing numeric components.
//! Source completeness is independent of the deliberately Partial game rules.
use super::{class_support_source as source, originals, tracked_release};
use poe_optimizer_core::{
    build_identity::BuildLineage, owned_build::ParameterValue, owned_definitions::*,
    owned_draft::DraftFinalization, owned_rules::*, owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{EffectDisposition, RuleScratch};
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
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

fn typed<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
fn selector(key: u64) -> ExternalSelector {
    ExternalSelector::Definition(ExternalOwnerSelector::Class {
        key: SourceComponent::Text(key.to_string()),
    })
}
fn class(key: u64, expected: &str) -> ClassDefId {
    let selector = selector(key);
    let rows: Vec<_> = source::verified_mapping()
        .entries
        .iter()
        .filter(|row| row.source == selector)
        .collect();
    assert_eq!(rows.len(), 1);
    let MappingOutcome::Mapped {
        target: SchemaSubject::Definition(DefinitionAddress::Class(class)),
        basis: MappingBasis::Exact,
    } = &rows[0].outcome
    else {
        panic!("exact Class source mapping")
    };
    assert_eq!(class.key().as_str(), expected);
    class.clone()
}
pub(super) fn declarations() -> Vec<SupportSourceDomainDeclaration> {
    source::SOURCES
        .iter()
        .map(|(key, expected)| SupportSourceDomainDeclaration {
            owner: SchemaSubject::Definition(class(*key, expected).address()),
            domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
        })
        .collect()
}
fn owner(subject: &SchemaSubject) -> &'static DefinitionRules {
    let rows: Vec<_> = tracked_release()
        .release()
        .assembled()
        .checked()
        .rules()
        .input()
        .owners
        .iter()
        .filter(|row| &row.owner == subject)
        .collect();
    assert_eq!(rows.len(), 1);
    rows[0]
}
fn program<'a>(owner: &'a DefinitionRules, name: &str) -> &'a RuleProgram {
    let rows: Vec<_> = owner
        .programs
        .members
        .iter()
        .filter(|program| program.id.as_str() == name)
        .collect();
    assert_eq!(rows.len(), 1);
    rows[0]
}
fn complete_empty<T>(set: &DeclaredSet<T>) -> bool {
    set.is_complete() && set.members.is_empty()
}
fn assert_partial(owner: &DefinitionRules) {
    let SchemaClosure::Partial { gaps } = &owner.programs.closure else {
        panic!("source-domain completeness must not close numerical rules")
    };
    assert_eq!(gaps.len(), 1);
    assert_eq!(gaps[0].subject, owner.owner);
    assert_eq!(gaps[0].facet, SchemaFacet::GameRules);
    assert_eq!(gaps[0].code.as_str(), "tree-game-rules-not-converted");
}

fn check_class_structure() {
    let expected = declarations();
    assert_eq!(expected.len(), 8);
    let release = tracked_release().release();
    let recipe = release.assembled().checked();
    let providers = &recipe
        .rules()
        .input()
        .support_discovery
        .as_ref()
        .unwrap()
        .providers;
    let mut classes = BTreeSet::new();
    for ((key, id), declaration) in source::SOURCES.iter().zip(&expected) {
        let class = class(*key, id);
        assert!(classes.insert(class.clone()));
        assert_eq!(
            release.mapping().lookup(&selector(*key)),
            Some(&MappingOutcome::Mapped {
                target: declaration.owner.clone(),
                basis: MappingBasis::Exact,
            })
        );
        assert_eq!(
            providers
                .iter()
                .filter(|row| row.owner == declaration.owner)
                .collect::<Vec<_>>(),
            vec![declaration]
        );
        let DefinitionDescriptor::Class(definition) =
            recipe.schema().lookup_definition(&class.address()).unwrap()
        else {
            panic!("Class descriptor")
        };
        let SchemaState::Known(schema) = &definition.schema else {
            panic!("known Class schema")
        };
        assert_eq!(
            (schema.level.minimum.get(), schema.level.maximum.get()),
            (1, 100)
        );
        assert!(schema.ascendancies.is_complete());
        assert!(schema.implicit_passives.is_complete());
        let slots = &schema.declarations;
        assert!(complete_empty(&slots.parameters));
        assert!(complete_empty(&slots.choices));
        assert!(complete_empty(&slots.grants));
        assert!(complete_empty(&slots.actors));
        assert!(complete_empty(&slots.skill_grants));
        assert!(complete_empty(&slots.outputs));
        assert!(complete_empty(&slots.sockets));
        let rules = owner(&declaration.owner);
        assert_partial(rules);
        assert_eq!(
            rules
                .programs
                .members
                .iter()
                .map(|program| program.id.as_str())
                .collect::<Vec<_>>(),
            ["class-base-contributions", "intrinsic-attack-baseline"]
        );
    }
    let actual: BTreeSet<_> = recipe
        .schema()
        .input()
        .definitions
        .iter()
        .filter_map(|row| match row {
            DefinitionDescriptor::Class(row) => Some(row.id.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(actual, classes);
    assert_eq!(
        providers
            .iter()
            .filter(|row| matches!(
                row.owner,
                SchemaSubject::Definition(DefinitionAddress::Class(_))
            ))
            .count(),
        classes.len()
    );
    assert_eq!(release.mapping().lookup(&selector(0)), None);
    assert!(
        !source::verified_mapping()
            .entries
            .iter()
            .any(|row| row.source == selector(0))
    );
    assert!(
        source::verified_classes()
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["integerId"] != 0)
    );
    let unarmed = source::verified_unarmed()["classes"].as_array().unwrap();
    assert_eq!(unarmed.len(), classes.len() + 1);
    assert_eq!(
        unarmed.iter().filter(|row| row["class_key"] == "0").count(),
        1
    );
    assert_eq!(
        source::verified_attack_policy()["excluded_classes"],
        serde_json::json!(["0"])
    );
}

type Expected = BTreeMap<StatDefId, (UnitDefId, f64)>;

fn literal<'a>(program: &'a RuleProgram, id: &OwnedDefinitionKey) -> &'a ParameterValue {
    let rows: Vec<_> = program.nodes.iter().filter(|node| &node.id == id).collect();
    assert_eq!(rows.len(), 1);
    let RuleExpression::Literal { value } = &rows[0].expression else {
        panic!("literal Class component")
    };
    value
}
fn assert_component(
    owner: &DefinitionRules,
    program: &RuleProgram,
    expected: &Expected,
    additive: bool,
    scratch: &mut RuleScratch,
) {
    assert_partial(owner);
    assert_eq!(program.context, RuleEntityKind::Actor);
    assert!(program.reads.is_empty());
    assert_eq!(program.effects.len(), expected.len());
    assert_eq!(program.nodes.len(), if additive { 3 } else { 4 });
    assert!(program.nodes.iter().all(|node| matches!(
        node.expression,
        RuleExpression::Literal {
            value: ParameterValue::Quantity(_)
        }
    )));
    let mut actual = BTreeMap::new();
    let mut used = BTreeSet::new();
    for effect in &program.effects {
        assert!(effect.when.is_none());
        let (stat, value) = match &effect.effect {
            RuleEffectKind::Contribute {
                entity: RuleEntity::Player,
                stat,
                contribution: ContributionKind::Add,
                value,
            } if additive => (stat, value),
            RuleEffectKind::Derive {
                entity: RuleEntity::Player,
                stat,
                value,
            } if !additive => (stat, value),
            other => panic!("unexpected Class component: {other:?}"),
        };
        let ParameterValue::Quantity(amount) = literal(program, value) else {
            panic!("typed quantity")
        };
        assert!(
            actual
                .insert(stat.clone(), (amount.unit().clone(), amount.value()))
                .is_none(),
            "duplicate contribution or baseline destination"
        );
        used.insert(value);
    }
    assert_eq!(&actual, expected);
    assert_eq!(used.len(), program.nodes.len());
    let recipe = tracked_release().release().assembled().checked();
    let compiled = recipe.compiled_rules();
    let fresh = compiled
        .evaluate(
            &owner.owner,
            &program.id,
            &[],
            recipe.schema().as_ref(),
            &mut compiled.new_scratch(),
        )
        .unwrap();
    let reused = compiled
        .evaluate(
            &owner.owner,
            &program.id,
            &[],
            recipe.schema().as_ref(),
            scratch,
        )
        .unwrap();
    assert_eq!(reused, fresh);
    assert_eq!(reused.owner, owner.owner);
    assert_eq!(reused.owner_programs_closure, owner.programs.closure);
    assert_eq!(reused.effects.len(), expected.len());
    for (effect, original) in reused.effects.iter().zip(&program.effects) {
        assert_eq!(effect.id, original.id);
        assert_eq!(effect.effect, original.effect);
        let value = match &original.effect {
            RuleEffectKind::Contribute { value, .. } | RuleEffectKind::Derive { value, .. } => {
                value
            }
            _ => unreachable!(),
        };
        assert_eq!(
            effect.disposition,
            EffectDisposition::Applied {
                value: literal(program, value).clone()
            }
        );
    }
}

fn check_class_attributes() {
    // Independent complete source vectors in Strength/Dexterity/Intelligence order.
    let vectors = [
        (1, [7, 7, 15]),
        (10, [7, 11, 11]),
        (11, [11, 7, 11]),
        (2, [7, 15, 7]),
        (6, [15, 7, 7]),
        (7, [7, 7, 15]),
        (8, [7, 15, 7]),
        (9, [11, 11, 7]),
    ];
    let bindings = source::verified_count_bindings();
    let unit: UnitDefId = typed(&bindings["unit"]);
    assert_eq!(unit.key().as_str(), "def.000000000000295a");
    let policy = source::verified_class_policy();
    assert_eq!(policy["fields"].as_array().unwrap().len(), 3);
    assert_eq!(bindings["passes"].as_array().unwrap().len(), 2);
    let recipe = tracked_release().release().assembled().checked();
    let mut scratch = recipe.compiled_rules().new_scratch();
    for (key, values) in vectors.iter().chain(vectors.iter().rev()) {
        let raw: Vec<_> = source::verified_classes()
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["integerId"].as_u64() == Some(*key))
            .collect();
        assert_eq!(raw.len(), 1);
        let mut expected = BTreeMap::new();
        assert_eq!(values.iter().sum::<i64>(), 29);
        for (lane, field) in ["base_str", "base_dex", "base_int"].into_iter().enumerate() {
            assert_eq!(raw[0][field].as_i64(), Some(values[lane]));
            assert_eq!(policy["fields"][lane]["source_field"], field);
            assert_eq!(
                policy["fields"][lane]["stat"],
                bindings["final_attributes"][lane]
            );
            for (pass, name) in ["first", "second"].into_iter().enumerate() {
                assert_eq!(bindings["passes"][pass]["id"], name);
                assert_eq!(
                    bindings["passes"][pass]["inputs"].as_array().unwrap().len(),
                    3
                );
                let stat: StatDefId = typed(&bindings["passes"][pass]["inputs"][lane]);
                assert_eq!(
                    stat.key().as_str(),
                    format!("def.{:016x}", 0x331b + pass * 3 + lane)
                );
                assert!(
                    expected
                        .insert(stat, (unit.clone(), values[lane] as f64))
                        .is_none()
                );
            }
        }
        let (_, id) = source::SOURCES.iter().find(|(id, _)| id == key).unwrap();
        let owner = owner(&SchemaSubject::Definition(class(*key, id).address()));
        assert_component(
            owner,
            program(owner, "class-base-contributions"),
            &expected,
            true,
            &mut scratch,
        );
    }
}

fn check_class_unarmed() {
    let maxima = [
        (1, 5.0),
        (10, 5.0),
        (11, 6.0),
        (2, 5.0),
        (6, 8.0),
        (7, 5.0),
        (8, 5.0),
        (9, 6.0),
    ];
    let fields = [
        ("AttackRate", "1d35", "1d39", 1.65),
        ("CritChance", "1d36", "0002", 5.0),
        ("PhysicalMin", "1d37", "1d3a", 2.0),
        ("PhysicalMax", "1d38", "1d3a", 0.0),
    ];
    let policy = source::verified_attack_policy();
    assert_eq!(policy["constants"]["type"], "None");
    assert_eq!(policy["fields"].as_array().unwrap().len(), fields.len());
    let recipe = tracked_release().release().assembled().checked();
    let mut scratch = recipe.compiled_rules().new_scratch();
    for (key, max) in maxima.iter().chain(maxima.iter().rev()) {
        let raw: Vec<_> = source::verified_unarmed()["classes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["class_key"].as_str().unwrap().parse::<u64>().unwrap() == *key)
            .collect();
        assert_eq!(raw.len(), 1);
        assert_eq!(raw[0]["fields"]["type"], "None");
        let mut expected = BTreeMap::new();
        for (field, stat_key, unit_key, value) in fields {
            let value = if field == "PhysicalMax" { *max } else { value };
            assert_eq!(raw[0]["fields"][field].as_f64(), Some(value));
            let rows: Vec<_> = policy["fields"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["source_field"] == field)
                .collect();
            assert_eq!(rows.len(), 1);
            let stat: StatDefId = typed(&rows[0]["stat"]);
            let unit: UnitDefId = typed(&rows[0]["unit"]);
            assert_eq!(stat.key().as_str(), format!("def.000000000000{stat_key}"));
            assert_eq!(unit.key().as_str(), format!("def.000000000000{unit_key}"));
            assert!(expected.insert(stat, (unit, value)).is_none());
        }
        let (_, id) = source::SOURCES.iter().find(|(id, _)| id == key).unwrap();
        let owner = owner(&SchemaSubject::Definition(class(*key, id).address()));
        assert_component(
            owner,
            program(owner, "intrinsic-attack-baseline"),
            &expected,
            false,
            &mut scratch,
        );
    }
}

pub(super) fn check_accounted_classes() {
    check_class_structure();
    check_class_attributes();
    check_class_unarmed();
}

#[test]
fn eight_exact_class_domains_preserve_sixteen_programs_and_exclude_source_zero() {
    check_class_structure();
}
#[test]
fn all_class_attributes_execute_both_count_passes_with_exact_channels_and_reused_scratch() {
    check_class_attributes();
}
#[test]
fn all_unarmed_components_preserve_injected_values_units_and_partial_coverage() {
    check_class_unarmed();
}

#[test]
fn original_presets_resolve_classes_and_original_five_keeps_its_selected_class_and_pending_inputs()
{
    let release = tracked_release().release();
    let recipe = release.assembled().checked();
    let classes: BTreeSet<_> = source::SOURCES
        .iter()
        .map(|(key, id)| class(*key, id))
        .collect();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for case in 1..=5 {
        let xml = fs::read(root.join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        )))
        .unwrap();
        let imported = ImportedBuildInstance::from_decoded(
            decode_build(&xml).unwrap(),
            BuildLineage::from_bytes([case; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
        let query_set = release
            .query_sets()
            .iter()
            .find(|set| set.name.as_str() == format!("original-{case:02}"))
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
            &query_set.queries,
            Default::default(),
        )
        .unwrap();
        let draft = normalized.draft();
        let snapshot = draft.clone();
        assert!(!draft.input().character_presets.members.is_empty());
        for preset in &draft.input().character_presets.members {
            assert!(classes.contains(&preset.class.to_resolved().unwrap()));
        }
        if case == 5 {
            let selection = originals::saved(&evidence, &normalized);
            let selected = draft
                .input()
                .character_presets
                .members
                .iter()
                .find(|row| row.id == selection.build.character)
                .unwrap();
            assert_eq!(
                selected.class.to_resolved(),
                Some(class(7, "def.0000000000000a23"))
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
        }
        assert_eq!(draft, &snapshot);
    }
}
