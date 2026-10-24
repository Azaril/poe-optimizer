//! Maintained class-view component laws. The retained CLI test still owns the
//! isolated legacy whole-plan activation/Partial law until real readiness exists.
use super::{originals, tracked_release, view_support_source as source};
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_build::{LoadoutScope, ParameterValue},
    owned_definitions::*,
    owned_draft::{DraftAllocationAccess, DraftFinalization},
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{EffectDisposition, RuleFact, RuleScratch};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_mapping::{
        ExternalOwnerSelector, ExternalSelector, MappingBasis, MappingOutcome, SourceComponent,
    },
    owned_normalize::{NormalizationArtifacts, normalize_fresh},
    owned_passive_views::{ViewContribution, ViewSelector},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
    owned_tree_policy::TreeTokenRole,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{fs, path::Path};

const OWNED: [&str; 7] = ["0c76", "0ce6", "0dd6", "13c0", "1585", "1694", "1951"];
fn typed<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
fn node(token: &str) -> PassiveNodeDefId {
    let rows: Vec<_> = source::verified_tree()
        .content
        .tokens
        .iter()
        .filter(|r| r.token == token)
        .collect();
    assert_eq!(rows.len(), 1);
    let TreeTokenRole::Allocation { node, .. } = &rows[0].role else {
        panic!("physical allocation")
    };
    let selector = ExternalSelector::Definition(ExternalOwnerSelector::PassiveNode {
        tree_version: SourceComponent::Text(source::verified_catalog().tree_version.clone()),
        node_id: SourceComponent::Text(token.into()),
        view: SourceComponent::Missing,
    });
    let entries: Vec<_> = source::verified_mapping()
        .entries
        .iter()
        .filter(|e| e.source == selector)
        .collect();
    assert_eq!(entries.len(), 1);
    assert!(
        matches!(&entries[0].outcome, MappingOutcome::Mapped {target:SchemaSubject::Definition(DefinitionAddress::PassiveNode(id)),basis:MappingBasis::Exact} if id == node)
    );
    let position = source::SOURCE_IDS.iter().position(|s| *s == token).unwrap();
    assert_eq!(
        node.key().as_str(),
        format!("def.000000000000{}", OWNED[position])
    );
    node.clone()
}
pub(super) fn declarations() -> Vec<SupportSourceDomainDeclaration> {
    source::SOURCE_IDS
        .into_iter()
        .map(|id| SupportSourceDomainDeclaration {
            owner: SchemaSubject::Definition(node(id).address()),
            domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
        })
        .collect()
}
fn class(key: &str) -> ClassDefId {
    source::verified_tree()
        .content
        .classes
        .iter()
        .find(|c| c.key == key)
        .unwrap()
        .class
        .clone()
}
fn program(id: &PassiveNodeDefId) -> &'static RuleProgram {
    let recipe = tracked_release().release().assembled().checked();
    let owner = recipe
        .rules()
        .input()
        .owners
        .iter()
        .find(|r| r.owner == SchemaSubject::Definition(id.address()))
        .unwrap();
    assert!(owner.programs.is_complete());
    assert_eq!(owner.programs.members.len(), 1);
    let p = &owner.programs.members[0];
    assert_eq!(p.id.as_str(), "passive-view");
    p
}
fn quantity(value: f64, unit: &UnitDefId) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(value, unit.clone()).unwrap())
}
fn expected(id: &str, witch: bool) -> Vec<ViewContribution> {
    let row = source::verified_policy()
        .nodes
        .iter()
        .find(|r| r.node == id)
        .unwrap();
    assert_eq!(row.views.len(), 1);
    assert_eq!(row.views[0].selector, "Witch");
    assert_eq!(row.views[0].when, ViewSelector::Class { key: "1".into() });
    let historical = if witch {
        &row.views[0].effects.contributions
    } else {
        &row.default.contributions
    };
    let count = source::verified_count_bindings();
    let intelligence: StatDefId = typed(&count["final_attributes"][2]);
    let unit: UnitDefId = typed(&count["unit"]);
    let mut result = Vec::new();
    for effect in historical {
        if effect.stat == intelligence {
            assert_eq!(id, "51184");
            assert_eq!(effect.contribution, ContributionKind::Add);
            assert!(matches!(&effect.value,ParameterValue::Integer(n) if n.get() == 10));
            for pass in [0, 1] {
                result.push(ViewContribution {
                    stat: typed(&count["passes"][pass]["inputs"][2]),
                    contribution: ContributionKind::Add,
                    value: quantity(10.0, &unit),
                });
            }
        } else {
            result.push(effect.clone());
        }
    }
    result
}
fn effects(
    id: &PassiveNodeDefId,
    class: &ClassDefId,
    scratch: &mut RuleScratch,
) -> Vec<ViewContribution> {
    let recipe = tracked_release().release().assembled().checked();
    let p = program(id);
    let facts: Vec<_> = p
        .reads
        .iter()
        .map(|r| {
            let RuleReadSource::CharacterClassIs { class: expected } = &r.source else {
                panic!("exact class fact")
            };
            RuleFact {
                read: r.id.clone(),
                value: ParameterValue::Boolean(expected == class),
            }
        })
        .collect();
    recipe
        .compiled_rules()
        .evaluate(
            &SchemaSubject::Definition(id.address()),
            &p.id,
            &facts,
            recipe.schema().as_ref(),
            scratch,
        )
        .unwrap()
        .effects
        .into_iter()
        .filter_map(|e| match e.disposition {
            EffectDisposition::Inactive => None,
            EffectDisposition::Applied { value } => {
                let RuleEffectKind::Contribute {
                    entity: RuleEntity::Player,
                    stat,
                    contribution,
                    ..
                } = e.effect
                else {
                    panic!("player contribution")
                };
                Some(ViewContribution {
                    stat,
                    contribution,
                    value,
                })
            }
            other => panic!("known class must resolve: {other:?}"),
        })
        .collect()
}
fn complete_empty<T>(s: &DeclaredSet<T>) -> bool {
    s.is_complete() && s.members.is_empty()
}
fn check_current_output_families() {
    let recipe = tracked_release().release().assembled().checked();
    let declarations = declarations();
    let inventory = &recipe
        .rules()
        .input()
        .support_discovery
        .as_ref()
        .unwrap()
        .providers;
    for (id, declaration) in source::SOURCE_IDS.into_iter().zip(&declarations) {
        assert_eq!(
            inventory
                .iter()
                .filter(|r| r.owner == declaration.owner)
                .collect::<Vec<_>>(),
            vec![declaration]
        );
        let node = node(id);
        let SchemaLookup::Known(schema) = recipe.schema().definition(&node) else {
            panic!("known passive")
        };
        assert!(schema.pools.is_complete() && schema.adjacent.is_complete());
        let d = &schema.declarations;
        assert!(
            complete_empty(&d.parameters)
                && complete_empty(&d.choices)
                && complete_empty(&d.grants)
                && complete_empty(&d.actors)
                && complete_empty(&d.skill_grants)
                && complete_empty(&d.outputs)
                && complete_empty(&d.sockets)
        );
        let p = program(&node);
        assert_eq!(p.context, RuleEntityKind::Actor);
        assert_eq!(p.reads.len(), 1);
        assert_eq!(p.reads[0].value_type, ComputedValueType::Boolean);
        assert_eq!(
            p.reads[0].source,
            RuleReadSource::CharacterClassIs { class: class("1") }
        );
        assert!(p.nodes.iter().all(|n| matches!(
            &n.expression,
            RuleExpression::Read { .. }
                | RuleExpression::Literal {
                    value: ParameterValue::Boolean(_) | ParameterValue::Quantity(_)
                }
                | RuleExpression::Any { .. }
                | RuleExpression::Not { .. }
                | RuleExpression::Select { .. }
        )));
        assert!(p.effects.iter().all(|e| e.when.is_some()
            && matches!(
                &e.effect,
                RuleEffectKind::Contribute {
                    entity: RuleEntity::Player,
                    ..
                }
            )));
        assert_eq!(
            p.effects.len(),
            expected(id, false).len() + expected(id, true).len()
        );
    }
    for expected in source::verified_statistics() {
        let Some(DefinitionDescriptor::Stat(actual)) =
            recipe.schema().lookup_definition(&expected.id.address())
        else {
            panic!("known stat")
        };
        assert_eq!(actual, expected);
    }
    let count = source::verified_count_bindings();
    let unit: UnitDefId = typed(&count["unit"]);
    assert_eq!(unit.key().as_str(), "def.000000000000295a");
    for (pass, key) in [(0, "def.000000000000331d"), (1, "def.0000000000003320")] {
        let stat: StatDefId = typed(&count["passes"][pass]["inputs"][2]);
        assert_eq!(stat.key().as_str(), key);
        let SchemaLookup::Known(schema) = recipe.schema().definition(&stat) else {
            panic!("Count input")
        };
        assert_eq!(
            schema.value,
            ComputedValueType::Quantity { unit: unit.clone() }
        );
        assert_eq!(schema.targets, [RuleEntityKind::Actor]);
    }
    let rows: Vec<_> = source::verified_count_programs()["programs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| {
            typed::<SchemaSubject>(&r["owner"])
                == SchemaSubject::Definition(node("51184").address())
        })
        .collect();
    assert_eq!(rows.len(), 1);
    let after: RuleProgram = typed(&rows[0]["after"]);
    assert_eq!(after.effects.len(), 7);
    assert!(after.nodes.iter().all(|n| !matches!(
        &n.expression,
        RuleExpression::Literal {
            value: ParameterValue::Integer(_)
        }
    )));
    assert_eq!(declarations.len(), 7);
    for (index, declaration) in declarations.iter().enumerate() {
        assert!(
            declarations[..index]
                .iter()
                .all(|prior| prior.owner != declaration.owner)
        );
    }
}
fn check_both_whole_lists() {
    let recipe = tracked_release().release().assembled().checked();
    let mut scratch = recipe.compiled_rules().new_scratch();
    for id in source::SOURCE_IDS {
        let node = node(id);
        let p = program(&node);
        for (witch, class) in [(false, class("7")), (true, class("1")), (false, class("7"))] {
            assert_eq!(effects(&node, &class, &mut scratch), expected(id, witch));
        }
        let missing = recipe
            .compiled_rules()
            .evaluate(
                &SchemaSubject::Definition(node.address()),
                &p.id,
                &[],
                recipe.schema().as_ref(),
                &mut scratch,
            )
            .unwrap();
        assert_eq!(missing.effects.len(), p.effects.len());
        assert!(missing.effects.iter().all(|e|matches!(&e.disposition,EffectDisposition::Unresolved {input} if input == &p.reads[0].id)));
        assert_eq!(
            effects(&node, &class("1"), &mut scratch),
            expected(id, true)
        );
    }
    let default = effects(&node("22314"), &class("7"), &mut scratch);
    let witch = effects(&node("22314"), &class("1"), &mut scratch);
    assert_eq!(default.len(), 1);
    assert_eq!(witch.len(), 2);
    assert!(witch.iter().all(|e| e.stat != default[0].stat));
    let default = effects(&node("51184"), &class("7"), &mut scratch);
    let witch = effects(&node("51184"), &class("1"), &mut scratch);
    assert_eq!(default.len(), 3);
    assert_eq!(witch.len(), 4);
    assert_eq!(default[1..], witch[2..]);
    for (e, wanted) in [(&default[0], 20.0), (&witch[0], 16.0), (&witch[1], 16.0)] {
        let ParameterValue::Quantity(q) = &e.value else {
            panic!("Percent")
        };
        assert_eq!(q.value(), wanted);
    }
}
/// Both complete branches are checked against authenticated source contributions.
pub(super) fn check_accounted_views() {
    static CHECKED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    CHECKED.get_or_init(|| {
        check_current_output_families();
        check_both_whole_lists();
    });
}
#[test]
fn seven_exact_owners_keep_only_pure_class_contributions_and_current_count_channels() {
    check_current_output_families();
}
#[test]
fn both_whole_lists_execute_and_missing_class_never_selects_default_with_reused_scratch() {
    check_both_whole_lists();
}
#[test]
fn maintained_receivers_keep_exact_target_and_missing_input_semantics() {
    let recipe = tracked_release().release().assembled().checked();
    let mut scratch = recipe.compiled_rules().new_scratch();
    let policy = source::verified_policy();
    assert_eq!(policy.receivers.len(), 2);
    for expected in &policy.receivers {
        let rows: Vec<_> = recipe
            .rules()
            .input()
            .receivers
            .members
            .iter()
            .filter(|r| r.id == expected.id)
            .collect();
        assert_eq!(rows, vec![expected]);
        let owner = SchemaSubject::Definition(expected.stat.address());
        let body = recipe
            .rules()
            .input()
            .owners
            .iter()
            .find(|r| r.owner == owner)
            .unwrap();
        let p = body
            .programs
            .members
            .iter()
            .find(|p| p.id == expected.program)
            .unwrap();
        assert!(body.programs.is_complete());
        assert_eq!(p.context, RuleEntityKind::Actor);
        assert_eq!(p.reads.len(), 1);
        assert_eq!(p.nodes.len(), 1);
        assert_eq!(p.effects.len(), 1);
        assert!(p.effects[0].when.is_none());
        assert!(
            matches!(&p.nodes[0].expression,RuleExpression::Read {input} if input==&p.reads[0].id)
        );
        assert!(
            matches!(&p.effects[0].effect,RuleEffectKind::Derive {entity:RuleEntity::Current,stat,value} if stat==&expected.stat && value==&p.nodes[0].id)
        );
        let historical = policy
            .receiver_rules
            .iter()
            .find(|r| r.owner == owner)
            .unwrap()
            .programs
            .members
            .iter()
            .find(|r| r.id == p.id)
            .unwrap();
        assert_eq!(p.reads[0].source, historical.reads[0].source);
        assert_eq!(p.reads[0].value_type, historical.reads[0].value_type);
        let ComputedValueType::Quantity { unit } = &p.reads[0].value_type else {
            panic!("percentage receiver")
        };
        for amount in [0.0, 8.0, 20.0, 0.0] {
            let value = quantity(amount, unit);
            let result = recipe
                .compiled_rules()
                .evaluate(
                    &owner,
                    &p.id,
                    &[RuleFact {
                        read: p.reads[0].id.clone(),
                        value: value.clone(),
                    }],
                    recipe.schema().as_ref(),
                    &mut scratch,
                )
                .unwrap();
            assert_eq!(
                result.effects[0].disposition,
                EffectDisposition::Applied { value }
            );
        }
        let missing = recipe
            .compiled_rules()
            .evaluate(&owner, &p.id, &[], recipe.schema().as_ref(), &mut scratch)
            .unwrap();
        assert!(
            matches!(&missing.effects[0].disposition,EffectDisposition::Unresolved {input} if input==&p.reads[0].id)
        );
    }
}
#[test]
fn saved_sorceress_uses_4739_default_with_existing_access_and_pending_obligations() {
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
        .find(|s| s.name.as_str() == "original-05")
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
    let selected = &draft
        .input()
        .allocation_presets
        .members
        .iter()
        .find(|p| p.id == selection.build.allocations)
        .unwrap()
        .allocations
        .members;
    let allocation = draft
        .input()
        .allocations
        .members
        .iter()
        .find(|a| selected.contains(&a.id) && a.node.to_resolved() == Some(node("4739")))
        .unwrap();
    // The maintained independent-access proof already admits this ordinary
    // allocation. Source classification changes neither that access nor the
    // separate unresolved build obligations checked below.
    assert_eq!(allocation.access, DraftAllocationAccess::Ordinary);
    assert_eq!(allocation.scope.to_resolved(), Some(LoadoutScope::Shared));
    let selected_class = draft
        .input()
        .character_presets
        .members
        .iter()
        .find(|p| p.id == selection.build.character)
        .unwrap()
        .class
        .to_resolved()
        .unwrap();
    assert_eq!(selected_class, class("7"));
    let mut scratch = recipe.compiled_rules().new_scratch();
    let actual = effects(&node("4739"), &selected_class, &mut scratch);
    assert_eq!(actual, expected("4739", false));
    assert_eq!(actual.len(), 1);
    let ParameterValue::Quantity(q) = &actual[0].value else {
        panic!("Spell percentage")
    };
    assert_eq!(q.value(), 10.0);
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
        matches!(checked.finalization(),DraftFinalization::Pending {issues,..} if issues.len()==5)
    );
    assert_eq!(draft, &snapshot);
    assert_eq!(checked.draft_digest(), normalized.sidecar().draft);
}
