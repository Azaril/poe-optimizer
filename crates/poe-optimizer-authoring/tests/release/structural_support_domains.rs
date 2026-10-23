//! Six intrinsic source domains retain positive supply chains and every unrelated
//! unresolved facet. Existing checked rules execute; no predecessor is rebuilt.
use super::{originals, structural_support_source as source, tracked_release};
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_build::{DeclaredSlot, LoadoutScope, ParameterValue},
    owned_definitions::*,
    owned_draft::{DraftAllocationAccess, DraftField, DraftFinalization},
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_engine::owned_rules::EffectDisposition;
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

const EXCLUDED: [&str; 6] = ["0a6d", "0bb3", "0e15", "0f2a", "173f", "1760"];
fn typed<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
fn selector(id: u64) -> ExternalSelector {
    ExternalSelector::Definition(ExternalOwnerSelector::PassiveNode {
        tree_version: SourceComponent::Text(source::catalog().tree_version.clone()),
        node_id: SourceComponent::Text(id.to_string()),
        view: SourceComponent::Missing,
    })
}
fn node(source_id: u64, owned: &str) -> PassiveNodeDefId {
    let selector = selector(source_id);
    let rows: Vec<_> = source::mapping()
        .entries
        .iter()
        .filter(|row| row.source == selector)
        .collect();
    assert_eq!(rows.len(), 1);
    let MappingOutcome::Mapped {
        target: SchemaSubject::Definition(DefinitionAddress::PassiveNode(node)),
        basis: MappingBasis::Exact,
    } = &rows[0].outcome
    else {
        panic!("exact physical node mapping")
    };
    assert_eq!(node.key().as_str(), owned);
    node.clone()
}
pub(super) fn declarations() -> Vec<SupportSourceDomainDeclaration> {
    source::SOURCES
        .iter()
        .map(|(id, owned)| SupportSourceDomainDeclaration {
            owner: SchemaSubject::Definition(node(*id, owned).address()),
            domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
        })
        .collect()
}
fn current_definition(address: &DefinitionAddress) -> &'static DefinitionDescriptor {
    tracked_release()
        .release()
        .assembled()
        .checked()
        .schema()
        .lookup_definition(address)
        .unwrap()
}
fn current_owner(owner: &SchemaSubject) -> &'static DefinitionRules {
    let rows: Vec<_> = tracked_release()
        .release()
        .assembled()
        .checked()
        .rules()
        .input()
        .owners
        .iter()
        .filter(|row| &row.owner == owner)
        .collect();
    assert_eq!(rows.len(), 1);
    rows[0]
}
fn current_slot(address: &SlotAddress) -> &'static SlotDescriptor {
    let rows: Vec<_> = tracked_release()
        .release()
        .assembled()
        .checked()
        .schema()
        .input()
        .slots
        .iter()
        .filter(|row| &row.address() == address)
        .collect();
    assert_eq!(rows.len(), 1);
    rows[0]
}
fn prior_definitions() -> Vec<DefinitionDescriptor> {
    typed(&source::dependencies()["definitions"])
}
fn exact_refinement(before: &DefinitionDescriptor, after: &DefinitionDescriptor) -> bool {
    let (DefinitionDescriptor::PassiveNode(old), DefinitionDescriptor::PassiveNode(next)) =
        (before, after)
    else {
        return false;
    };
    let (SchemaState::Known(old_schema), SchemaState::Known(next_schema)) =
        (&old.schema, &next.schema)
    else {
        return false;
    };
    let mut restored = next.clone();
    let SchemaState::Known(schema) = &mut restored.schema else {
        unreachable!()
    };
    macro_rules! restore {
        ($field:ident) => {
            if old_schema.declarations.$field.is_complete()
                || !next_schema.declarations.$field.is_complete()
                || old_schema.declarations.$field.members != next_schema.declarations.$field.members
            {
                return false;
            }
            schema.declarations.$field.closure = old_schema.declarations.$field.closure.clone();
        };
    }
    restore!(grants);
    restore!(actors);
    restore!(skill_grants);
    restored == *old
}

#[test]
fn six_exact_domains_preserve_three_closure_refinements_and_partial_numerical_coverage() {
    let declarations = declarations();
    let before = prior_definitions();
    let replacements: Vec<DefinitionDescriptor> = source::migration()["schema"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            assert_eq!(entry["kind"], "definition");
            typed(&entry["value"])
        })
        .collect();
    assert_eq!(before.len(), 6);
    assert_eq!(replacements.len(), before.len());
    assert_eq!(declarations.len(), before.len());
    let recipe = tracked_release().release().assembled().checked();
    let inventory = &recipe
        .rules()
        .input()
        .support_discovery
        .as_ref()
        .unwrap()
        .providers;
    let prior_owners: Vec<DefinitionRules> = typed(&source::dependencies()["owners"]);
    let mut nonempty = 0;
    for (index, old) in before.iter().enumerate() {
        let subject = SchemaSubject::Definition(old.address());
        let selector = selector(source::SOURCES[index].0);
        assert_eq!(
            tracked_release().release().mapping().lookup(&selector),
            Some(&MappingOutcome::Mapped {
                target: subject.clone(),
                basis: MappingBasis::Exact,
            })
        );
        assert_eq!(declarations[index].owner, subject);
        assert!(!declarations[..index].iter().any(|row| row.owner == subject));
        assert_eq!(
            inventory
                .iter()
                .filter(|row| row.owner == subject)
                .collect::<Vec<_>>(),
            vec![&declarations[index]]
        );
        let next = current_definition(&old.address());
        let retained: Vec<_> = replacements
            .iter()
            .filter(|row| row.address() == old.address())
            .collect();
        assert_eq!(retained, vec![next]);
        assert!(exact_refinement(old, next));
        let DefinitionDescriptor::PassiveNode(row) = next else {
            panic!("passive")
        };
        let SchemaState::Known(schema) = &row.schema else {
            panic!("known descriptor")
        };
        let d = &schema.declarations;
        assert!(d.grants.is_complete() && d.actors.is_complete() && d.skill_grants.is_complete());
        assert!(d.actors.members.is_empty());
        assert!(!d.parameters.is_complete() && !d.choices.is_complete());
        assert!(!d.outputs.is_complete() && !d.sockets.is_complete());
        nonempty += usize::from(!d.grants.members.is_empty())
            + usize::from(!d.actors.members.is_empty())
            + usize::from(!d.skill_grants.members.is_empty());
        let old_owner = prior_owners
            .iter()
            .find(|row| row.owner == subject)
            .unwrap();
        let current = current_owner(&subject);
        assert!(!current.programs.is_complete());
        assert_eq!(current.programs.closure, old_owner.programs.closure);
        assert_eq!(
            current
                .programs
                .members
                .iter()
                .map(|p| &p.id)
                .collect::<Vec<_>>(),
            old_owner
                .programs
                .members
                .iter()
                .map(|p| &p.id)
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(nonempty, 4, "two existing grants and two existing supplies");
    assert_eq!(source::evidence()["candidate_census_complete"], true);
    assert_eq!(source::evidence()["candidate_closure_complete"], false);
    let excluded: Vec<DefinitionDescriptor> =
        typed(&source::dependencies()["unresolved_definitions"]);
    assert_eq!(excluded.len(), EXCLUDED.len());
    for (old, suffix) in excluded.iter().zip(EXCLUDED) {
        assert_eq!(
            old.address().key().as_str(),
            format!("def.000000000000{suffix}")
        );
        assert_eq!(current_definition(&old.address()), old);
        assert!(
            !declarations
                .iter()
                .any(|row| row.owner == SchemaSubject::Definition(old.address()))
        );
        assert!(
            !inventory
                .iter()
                .any(|row| row.owner == SchemaSubject::Definition(old.address()))
        );
        let DefinitionDescriptor::PassiveNode(row) = old else {
            panic!("excluded passive")
        };
        let SchemaState::Known(schema) = &row.schema else {
            panic!("known excluded descriptor")
        };
        assert!(!schema.declarations.grants.is_complete());
        assert!(!schema.declarations.actors.is_complete());
        assert!(!schema.declarations.skill_grants.is_complete());
    }
}

type Passive = DefinitionEntry<PassiveNodeDefId, PassiveNodeSchema>;
type Grant = DefinitionEntry<DeclaredSlot<GrantSlotDefId>, GrantSlotSchema>;
type Supply = DefinitionEntry<DeclaredSlot<SkillGrantSlotDefId>, SkillGrantSlotSchema>;
type Skill = DefinitionEntry<SkillDefId, SkillSchema>;
struct Chain {
    passive: Passive,
    grant: Grant,
    supply: Supply,
    skill: Skill,
    raw_parameter: DeclaredSlot<ParameterSlotDefId>,
    program: &'static str,
}
fn chains() -> Vec<Chain> {
    let slots: Vec<SlotDescriptor> = typed(&source::dependencies()["slots"]);
    let skills: Vec<DefinitionDescriptor> = typed(&source::dependencies()["skills"]);
    assert_eq!(slots.len(), 4);
    assert_eq!(skills.len(), 2);
    for slot in &slots {
        assert_eq!(current_slot(&slot.address()), slot);
    }
    for skill in &skills {
        assert_eq!(current_definition(&skill.address()), skill);
    }
    [
        (
            13289,
            "def.0000000000000b34",
            "32d3",
            "32d2",
            "0322",
            "3261",
            "sand-tree-raw-supply",
        ),
        (
            32705,
            "def.00000000000010d4",
            "32d5",
            "32d4",
            "032c",
            "3263",
            "water-tree-raw-supply",
        ),
    ]
    .into_iter()
    .map(|(source_id, passive, grant, supply, skill, raw, program)| {
        let node = node(source_id, passive);
        let DefinitionDescriptor::PassiveNode(passive) = current_definition(&node.address()) else {
            panic!("passive")
        };
        let grant = slots
            .iter()
            .find_map(|slot| match slot {
                SlotDescriptor::Grant(row)
                    if row.id.slot.key().as_str() == format!("def.000000000000{grant}") =>
                {
                    Some(row.clone())
                }
                _ => None,
            })
            .unwrap();
        let supply = slots
            .iter()
            .find_map(|slot| match slot {
                SlotDescriptor::SkillGrant(row)
                    if row.id.slot.key().as_str() == format!("def.000000000000{supply}") =>
                {
                    Some(row.clone())
                }
                _ => None,
            })
            .unwrap();
        let skill = skills
            .iter()
            .find_map(|descriptor| match descriptor {
                DefinitionDescriptor::Skill(row)
                    if row.id.key().as_str() == format!("def.000000000000{skill}") =>
                {
                    Some(row.clone())
                }
                _ => None,
            })
            .unwrap();
        let raw_parameter = DeclaredSlot {
            declaration: SlotOwnerDefId::Skill(skill.id.clone()),
            slot: ParameterSlotDefId::new(
                skill.id.namespace().clone(),
                format!("def.000000000000{raw}").parse().unwrap(),
            ),
        };
        Chain {
            passive: passive.clone(),
            grant,
            supply,
            skill,
            raw_parameter,
            program,
        }
    })
    .collect()
}
fn valid_chain(chain: &Chain) -> bool {
    let (
        SchemaState::Known(passive),
        SchemaState::Known(grant),
        SchemaState::Known(supply),
        SchemaState::Known(_),
    ) = (
        &chain.passive.schema,
        &chain.grant.schema,
        &chain.supply.schema,
        &chain.skill.schema,
    )
    else {
        return false;
    };
    let owner = SlotOwnerDefId::PassiveNode(chain.passive.id.clone());
    passive.declarations.grants.is_complete()
        && passive.declarations.skill_grants.is_complete()
        && passive.declarations.actors.is_complete()
        && passive.declarations.grants.members == [chain.grant.id.clone()]
        && passive.declarations.skill_grants.members == [chain.supply.id.clone()]
        && passive.declarations.actors.members.is_empty()
        && chain.grant.id.declaration == owner
        && chain.supply.id.declaration == owner
        && grant.provider_roles == [ProviderRole::Allocation]
        && grant.target == GrantTarget::Skill(chain.supply.id.clone())
        && supply.skill == chain.skill.id
        && !supply.outputs.is_complete()
}
fn literal<'a>(program: &'a RuleProgram, id: &OwnedDefinitionKey) -> &'a ParameterValue {
    let nodes: Vec<_> = program.nodes.iter().filter(|row| &row.id == id).collect();
    assert_eq!(nodes.len(), 1);
    let RuleExpression::Literal { value } = &nodes[0].expression else {
        panic!("literal supplier")
    };
    value
}
fn check_supplier(program: &RuleProgram, chain: &Chain) {
    assert_eq!(program.id.as_str(), chain.program);
    assert_eq!(program.context, RuleEntityKind::Actor);
    assert!(program.reads.is_empty());
    assert_eq!(program.nodes.len(), 2);
    assert_eq!(program.effects.len(), 2);
    let mut projected = 0;
    let mut activated = 0;
    for effect in &program.effects {
        assert!(effect.when.is_none());
        match &effect.effect {
            RuleEffectKind::ProjectSkillParameter {
                skill,
                parameter,
                value,
            } => {
                projected += 1;
                assert_eq!(skill, &chain.supply.id);
                assert_eq!(parameter, &chain.raw_parameter);
                let ParameterValue::Quantity(value) = literal(program, value) else {
                    panic!("Count level")
                };
                assert_eq!(value.value(), 1.0);
                assert_eq!(value.unit().key().as_str(), "def.000000000000295a");
            }
            RuleEffectKind::ActivateGrant { slot, enabled } => {
                activated += 1;
                assert_eq!(slot, &chain.grant.id);
                assert_eq!(literal(program, enabled), &ParameterValue::Boolean(true));
            }
            _ => panic!("only the retained level projection and exact grant activation"),
        }
    }
    assert_eq!((projected, activated), (1, 1));
}

#[test]
fn both_barya_chains_execute_level_one_and_exact_grants_without_closing_descendants() {
    let recipe = tracked_release().release().assembled().checked();
    let mut scratch = recipe.compiled_rules().new_scratch();
    let chains = chains();
    for chain in chains.iter().chain(chains.iter().rev()) {
        assert!(valid_chain(chain));
        let SchemaState::Known(supply) = &chain.supply.schema else {
            unreachable!()
        };
        assert!(supply.outputs.members.is_empty());
        let SchemaClosure::Partial { gaps } = &supply.outputs.closure else {
            panic!("retained outputs gap")
        };
        assert_eq!(gaps.len(), 1);
        assert_eq!(
            gaps[0].subject,
            SchemaSubject::Definition(chain.passive.id.address())
        );
        assert_eq!(gaps[0].facet, SchemaFacet::InputSchema);
        assert_eq!(
            gaps[0].code.as_str(),
            "tree-djinn-generated-outputs-unconverted"
        );
        let permission = supply.preset_inputs.as_ref().unwrap();
        assert!(permission.parameters.is_complete());
        assert_eq!(permission.parameters.members.len(), 1);
        assert_eq!(
            permission.parameters.members[0].declaration,
            SlotOwnerDefId::Skill(chain.skill.id.clone())
        );
        assert_ne!(permission.parameters.members[0], chain.raw_parameter);
        let subject = SchemaSubject::Definition(chain.passive.id.address());
        let owner = current_owner(&subject);
        assert!(!owner.programs.is_complete());
        assert_eq!(owner.programs.members.len(), 1);
        let program = &owner.programs.members[0];
        check_supplier(program, chain);
        let report = recipe
            .compiled_rules()
            .evaluate(
                &subject,
                &program.id,
                &[],
                recipe.schema().as_ref(),
                &mut scratch,
            )
            .unwrap();
        assert_eq!(report.owner_programs_closure, owner.programs.closure);
        assert_eq!(report.effects.len(), 2);
        for (effect, expected) in report.effects.iter().zip(&program.effects) {
            assert_eq!(effect.effect, expected.effect);
            let input = match &expected.effect {
                RuleEffectKind::ProjectSkillParameter { value, .. } => value,
                RuleEffectKind::ActivateGrant { enabled, .. } => enabled,
                _ => unreachable!(),
            };
            assert_eq!(
                effect.disposition,
                EffectDisposition::Applied {
                    value: literal(program, input).clone()
                }
            );
        }
    }
}

#[test]
fn preservation_controls_reject_unrelated_closures_missing_members_and_crossed_chains() {
    let old = prior_definitions().remove(0);
    let next = current_definition(&old.address());
    assert!(exact_refinement(&old, next));
    for mutation in 0..6 {
        let mut changed = next.clone();
        let DefinitionDescriptor::PassiveNode(row) = &mut changed else {
            unreachable!()
        };
        let SchemaState::Known(schema) = &mut row.schema else {
            unreachable!()
        };
        match mutation {
            0 => schema.declarations.parameters.closure = SchemaClosure::Complete,
            1 => schema.declarations.outputs.closure = SchemaClosure::Complete,
            2 => schema.declarations.sockets.closure = SchemaClosure::Complete,
            3 => schema.declarations.grants.members.clear(),
            4 => schema.declarations.skill_grants.members.clear(),
            5 => {
                assert!(schema.adjacent.members.pop().is_some());
            }
            _ => unreachable!(),
        }
        assert!(!exact_refinement(&old, &changed));
    }
    let mut chains = chains();
    assert!(chains.iter().all(valid_chain));
    let other_skill = chains[1].skill.id.clone();
    let other_supply = chains[1].supply.id.clone();
    let SchemaState::Known(supply) = &mut chains[0].supply.schema else {
        unreachable!()
    };
    let saved_skill = supply.skill.clone();
    supply.skill = other_skill;
    assert!(!valid_chain(&chains[0]));
    let SchemaState::Known(supply) = &mut chains[0].supply.schema else {
        unreachable!()
    };
    supply.skill = saved_skill;
    assert!(valid_chain(&chains[0]));
    let SchemaState::Known(grant) = &mut chains[0].grant.schema else {
        unreachable!()
    };
    grant.target = GrantTarget::Skill(other_supply);
    assert!(!valid_chain(&chains[0]));
}

#[test]
fn saved_original_five_selects_all_six_ordinary_sources_and_keeps_five_input_obligations() {
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
    let query_set = release
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
        &query_set.queries,
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
        .find(|preset| preset.id == selection.build.allocations)
        .unwrap()
        .allocations
        .members;
    let expected: BTreeSet<_> = source::SOURCES
        .iter()
        .map(|(id, owned)| node(*id, owned))
        .collect();
    assert_eq!(expected.len(), 6);
    let actual: Vec<_> = draft
        .input()
        .allocations
        .members
        .iter()
        .filter(|a| selected.contains(&a.id))
        .filter(|a| {
            a.node
                .to_resolved()
                .is_some_and(|node| expected.contains(&node))
        })
        .collect();
    assert_eq!(actual.len(), expected.len());
    assert_eq!(
        actual
            .iter()
            .map(|a| a.node.to_resolved().unwrap())
            .collect::<BTreeSet<_>>(),
        expected
    );
    for allocation in actual {
        assert_eq!(allocation.access, DraftAllocationAccess::Ordinary);
        assert_eq!(
            allocation.scope,
            DraftField::Known {
                value: LoadoutScope::Shared
            }
        );
    }
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
    assert_eq!(draft, &snapshot);
    assert_eq!(checked.draft_digest(), normalized.sidecar().draft);
}
