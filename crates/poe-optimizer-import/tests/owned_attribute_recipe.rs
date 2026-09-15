//! Real finite catalog plus independent coefficient/choice laws. No Lua or build execution.
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition, RuleFact};
use poe_optimizer_import::{
    owned_attribute_recipe::*, owned_mapping::*, owned_recipe::*, owned_tree_catalog::*,
};
use serde::de::DeserializeOwned;
use std::{fs, path::PathBuf};

fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn integer(v: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(v).unwrap())
}
fn load<T: DeserializeOwned>(relative: &str) -> T {
    serde_json::from_slice(
        &fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../data/owned/poe2/3887ae68")
                .join(relative),
        )
        .unwrap(),
    )
    .unwrap()
}
struct Fixture {
    recipe: OwnedRecipeInput,
    mapping: MappingPackageInput,
    catalog: TreeCatalogInput,
    policy: AttributeRecipePolicy,
}
impl Fixture {
    fn new() -> Self {
        let mut recipe: OwnedRecipeInput = load("current/recipe.json");
        let mapping: MappingPackageInput = load("current/mapping.json");
        let catalog: TreeCatalogInput = load("tree/tree-catalog.json");
        let mut registry =
            OwnedIdRegistry::new(recipe.registry.clone(), Default::default()).unwrap();
        let mut lanes = Vec::new();
        for (source, coefficient) in catalog.attribute_options.iter().zip([5, 7, 11]) {
            let stat: StatDefId = registry.allocate_definition().unwrap();
            recipe
                .schema
                .definitions
                .push(DefinitionDescriptor::Stat(DefinitionEntry {
                    id: stat.clone(),
                    schema: SchemaState::Known(StatSchema {
                        value: ComputedValueType::Integer,
                        targets: vec![RuleEntityKind::Actor],
                    }),
                }));
            lanes.push(AttributeLanePolicy {
                key: source.key.clone(),
                expected_stats: source.stats.clone(),
                stat,
                value: integer(coefficient),
            });
        }
        recipe.registry = registry.input().clone();
        rebind_recipe(&mut recipe);
        let policy = AttributeRecipePolicy {
            schema_version: OWNED_ATTRIBUTE_RECIPE_VERSION,
            version: key("independent-attribute-policy"),
            expected_node_stats: vec!["+5 to any Attribute".into()],
            lanes,
        };
        Self {
            recipe,
            mapping,
            catalog,
            policy,
        }
    }
    fn checked(&self) -> (StagedOwnedRecipe, OwnedMappingIndex) {
        let base = assemble_owned_recipe(self.recipe.clone(), Default::default()).unwrap();
        let mut input = self.mapping.clone();
        input.registry = base.registry().identity().unwrap();
        input.definitions = base.schema().identity().clone();
        let mapping =
            OwnedMappingIndex::new(input, base.registry(), base.schema(), Default::default())
                .unwrap();
        (base, mapping)
    }
    fn compile(&self) -> Result<StagedAttributeRecipe, AttributeRecipeError> {
        let (base, mapping) = self.checked();
        compile_owned_attribute_recipe(
            &base,
            &mapping,
            &self.catalog,
            &self.policy,
            Default::default(),
        )
    }
    fn first_node_mut(&mut self) -> &mut TreeNodeInput {
        self.catalog
            .nodes
            .iter_mut()
            .find(|n| matches!(n.kind, TreeNodeKind::Attribute { .. }))
            .unwrap()
    }
}
fn rebind_recipe(input: &mut OwnedRecipeInput) {
    let schema = poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage::new(
        input.schema.clone(),
        Default::default(),
    )
    .unwrap();
    input.rules.definitions = schema.identity().clone();
    input.routing.definitions = schema.identity().clone();
}
fn bad(f: &Fixture, message: &str) {
    let error = f.compile().unwrap_err();
    assert!(error.to_string().contains(message), "{error}");
}
fn selected_program<'a>(
    input: &'a OwnedRecipeInput,
    address: &DefinitionAddress,
) -> &'a DefinitionRules {
    input
        .rules
        .owners
        .iter()
        .find(|o| o.owner == SchemaSubject::Definition(address.clone()))
        .unwrap()
}

#[test]
fn finite_catalog_closes_all_reviewed_attributes_and_preserves_other_data() {
    let f = Fixture::new();
    let (base, _) = f.checked();
    let out = f.compile().unwrap();
    assert_eq!(out.receipt.converted_nodes, 293);
    assert_eq!(out.refined.len(), 293);
    assert_eq!(out.receipt.changed_program_owners, 293);
    assert_eq!(out.successor.registry, f.recipe.registry);
    assert_eq!(out.successor.schema.slots, base.schema().input().slots);
    for old in &base.schema().input().definitions {
        let current = out
            .successor
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == old.address())
            .unwrap();
        if out.refined.contains(&old.address()) {
            let DefinitionDescriptor::PassiveNode(before) = old else {
                panic!("nonpassive refinement");
            };
            let DefinitionDescriptor::PassiveNode(after) = current else {
                unreachable!()
            };
            let (SchemaState::Known(before), SchemaState::Known(after)) =
                (&before.schema, &after.schema)
            else {
                unreachable!()
            };
            assert_eq!(before.pools, after.pools);
            assert_eq!(before.adjacent, after.adjacent);
            assert_eq!(
                before.declarations.choices.members,
                after.declarations.choices.members
            );
            assert!(after.declarations.parameters.is_complete());
            assert!(after.declarations.choices.is_complete());
            assert!(after.declarations.grants.is_complete());
            assert!(after.declarations.actors.is_complete());
            assert!(after.declarations.skill_grants.is_complete());
            assert!(after.declarations.outputs.is_complete());
            assert!(after.declarations.sockets.is_complete());
            assert!(
                selected_program(&out.successor, &old.address())
                    .programs
                    .is_complete()
            );
        } else {
            assert_eq!(current, old);
        }
    }
    for owner in &base.rules().input().owners {
        if !out
            .refined
            .iter()
            .any(|a| owner.owner == SchemaSubject::Definition(a.clone()))
        {
            assert_eq!(
                out.successor
                    .rules
                    .owners
                    .iter()
                    .find(|o| o.owner == owner.owner),
                Some(owner)
            );
        }
    }
    assert_eq!(out.successor.rules.receivers, f.recipe.rules.receivers);
    assert_ne!(
        out.receipt.before_definitions,
        out.receipt.after_definitions
    );
}

#[test]
fn exact_choice_selects_only_injected_stat_and_value_and_missing_stays_unknown() {
    let f = Fixture::new();
    let out = f.compile().unwrap();
    let checked = assemble_owned_recipe(out.successor.clone(), Default::default()).unwrap();
    let compiled = CompiledRulePackage::compile(
        checked.rules().input(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let owner = selected_program(&out.successor, &out.refined[0]);
    let p = &owner.programs.members[0];
    let RuleReadSource::Choice { slot } = &p.reads[0].source else {
        panic!("choice read");
    };
    let SchemaLookup::Known(choice) = checked.schema().slot(slot) else {
        unreachable!()
    };
    let ValueSchema::Option { allowed } = &choice.value else {
        unreachable!()
    };
    let mut scratch = compiled.new_scratch();
    for option in &allowed.members {
        let result = compiled
            .evaluate(
                &owner.owner,
                &p.id,
                &[RuleFact {
                    read: p.reads[0].id.clone(),
                    value: ParameterValue::Option(option.clone()),
                }],
                checked.schema(),
                &mut scratch,
            )
            .unwrap();
        let applied: Vec<_> = result
            .effects
            .iter()
            .filter(|e| matches!(e.disposition, EffectDisposition::Applied { .. }))
            .collect();
        assert_eq!(applied.len(), 1);
        assert_eq!(
            result
                .effects
                .iter()
                .filter(|e| matches!(e.disposition, EffectDisposition::Inactive))
                .count(),
            2
        );
        let RuleEffectKind::Contribute {
            stat,
            entity,
            contribution,
            ..
        } = &applied[0].effect
        else {
            panic!("contribution");
        };
        assert_eq!(*entity, RuleEntity::Player);
        assert_eq!(*contribution, ContributionKind::Add);
        let lane = f.policy.lanes.iter().find(|l| l.stat == *stat).unwrap();
        assert_eq!(
            applied[0].disposition,
            EffectDisposition::Applied {
                value: lane.value.clone()
            }
        );
    }
    let missing = compiled
        .evaluate(&owner.owner, &p.id, &[], checked.schema(), &mut scratch)
        .unwrap();
    assert!(
        missing
            .effects
            .iter()
            .all(|e| matches!(e.disposition, EffectDisposition::Unresolved { .. }))
    );
    // The compiler does not derive values from source text: every lane has a
    // deliberately different coefficient despite the source's shared +5 text.
    let coefficients: std::collections::BTreeSet<_> = f
        .policy
        .lanes
        .iter()
        .map(|l| match &l.value {
            ParameterValue::Integer(v) => v.get(),
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(coefficients, [5, 7, 11].into_iter().collect());
}

#[test]
fn unchanged_repeat_reuses_identity_and_has_no_refinement_and_policy_order_is_irrelevant() {
    let mut f = Fixture::new();
    let first = f.compile().unwrap();
    f.recipe = first.successor.clone();
    f.policy.lanes.reverse();
    f.catalog.nodes.reverse();
    f.catalog.attribute_options.reverse();
    let second = f.compile().unwrap();
    assert!(second.refined.is_empty());
    assert_eq!(second.receipt.changed_program_owners, 0);
    assert_eq!(first.successor, second.successor);
    assert_eq!(
        second.receipt.before_definitions,
        second.receipt.after_definitions
    );
}

#[test]
fn source_semantics_and_duplicate_or_missing_lanes_fail_closed() {
    let mut f = Fixture::new();
    f.first_node_mut().stats.push("unconverted effect".into());
    bad(&f, "unreviewed attribute node");
    let mut f = Fixture::new();
    f.first_node_mut().views.push(TreeStatViewInput {
        selector: "Witch".into(),
        stats: vec![],
    });
    bad(&f, "unreviewed attribute node");
    let mut f = Fixture::new();
    f.first_node_mut().unlock.push("other".into());
    bad(&f, "unreviewed attribute node");
    let mut f = Fixture::new();
    f.first_node_mut().kind = TreeNodeKind::Attribute {
        pool: TreePoolKind::Ascendancy,
    };
    bad(&f, "unreviewed attribute node");
    let mut f = Fixture::new();
    f.catalog.attribute_options[0]
        .stats
        .push("unconverted lane effect".into());
    bad(&f, "unreviewed source lane");
    let mut f = Fixture::new();
    f.policy.lanes[1] = f.policy.lanes[0].clone();
    bad(&f, "duplicate or empty reviewed lane");
    let mut f = Fixture::new();
    f.policy.lanes.pop();
    bad(&f, "lane membership");
    let mut f = Fixture::new();
    let duplicate = f.first_node_mut().clone();
    f.catalog.nodes.push(duplicate);
    bad(&f, "duplicate physical node");
}

#[test]
fn nonnumeric_wrong_scope_and_unknown_stat_policies_are_rejected() {
    let mut f = Fixture::new();
    f.policy.lanes[0].value = ParameterValue::Boolean(true);
    bad(&f, "must be numeric");
    let mut f = Fixture::new();
    let stat = f.policy.lanes[0].stat.clone();
    let DefinitionDescriptor::Stat(entry) = f
        .recipe
        .schema
        .definitions
        .iter_mut()
        .find(|d| d.address() == stat.address())
        .unwrap()
    else {
        unreachable!()
    };
    let SchemaState::Known(schema) = &mut entry.schema else {
        unreachable!()
    };
    schema.targets = vec![RuleEntityKind::Action];
    rebind_recipe(&mut f.recipe);
    bad(&f, "type or scope");
    let mut f = Fixture::new();
    f.policy.lanes[0].stat =
        StatDefId::parse(f.recipe.schema.namespace.clone(), "def.ffffffffffffffff").unwrap();
    bad(&f, "unknown target stat");
}

#[test]
fn prior_program_authority_and_unrelated_declaration_gaps_cannot_be_erased() {
    let mut f = Fixture::new();
    let first = f.compile().unwrap();
    f.recipe = first.successor;
    f.policy.lanes[0].value = integer(99);
    bad(&f, "cannot replace");
    let mut f = Fixture::new();
    let node = f.first_node_mut().key.clone();
    let (_, mapping) = f.checked();
    let selector = ExternalSelector::Definition(ExternalOwnerSelector::PassiveNode {
        tree_version: SourceComponent::Text(f.catalog.tree_version.clone()),
        node_id: SourceComponent::Text(node),
        view: SourceComponent::Missing,
    });
    let Some(MappingOutcome::Mapped {
        target: SchemaSubject::Definition(address),
        ..
    }) = mapping.lookup(&selector)
    else {
        unreachable!()
    };
    let DefinitionDescriptor::PassiveNode(entry) = f
        .recipe
        .schema
        .definitions
        .iter_mut()
        .find(|d| d.address() == *address)
        .unwrap()
    else {
        unreachable!()
    };
    let SchemaState::Known(schema) = &mut entry.schema else {
        unreachable!()
    };
    let SchemaClosure::Partial { gaps } = &mut schema.declarations.parameters.closure else {
        unreachable!()
    };
    gaps[0].code = key("separate-unreviewed-parameter-family");
    rebind_recipe(&mut f.recipe);
    bad(&f, "cannot replace");
}

#[test]
fn missing_mappings_and_stale_source_or_schema_bindings_are_rejected() {
    let mut f = Fixture::new();
    f.catalog.source.files[0].sha256 = "0".repeat(64);
    bad(&f, "binding differs");
    let mut f = Fixture::new();
    f.catalog
        .source
        .files
        .push(f.catalog.source.files[0].clone());
    bad(&f, "binding differs");
    let mut f = Fixture::new();
    let key = f.first_node_mut().key.clone();
    f.mapping.entries.retain(|e| !matches!(&e.source, ExternalSelector::Definition(ExternalOwnerSelector::PassiveNode { node_id: SourceComponent::Text(id), .. }) if id == &key));
    bad(&f, "unresolved exact selector");
    let f = Fixture::new();
    let (base, mapping) = f.checked();
    let mut stale_recipe = f.recipe.clone();
    let first = f.compile().unwrap();
    stale_recipe.schema = first.successor.schema;
    rebind_recipe(&mut stale_recipe);
    let changed = assemble_owned_recipe(stale_recipe, Default::default()).unwrap();
    assert!(matches!(
        compile_owned_attribute_recipe(
            &changed,
            &mapping,
            &f.catalog,
            &f.policy,
            Default::default()
        ),
        Err(AttributeRecipeError::Binding)
    ));
    assert_ne!(base.schema().identity(), changed.schema().identity());
}

#[test]
fn bounded_work_wire_and_row_limits_reject_before_publication() {
    let f = Fixture::new();
    let (base, mapping) = f.checked();
    for limits in [
        AttributeRecipeLimits {
            max_work: 1,
            ..Default::default()
        },
        AttributeRecipeLimits {
            max_nodes: 1,
            ..Default::default()
        },
        AttributeRecipeLimits {
            max_wire_bytes: 32,
            ..Default::default()
        },
        AttributeRecipeLimits {
            max_lanes: 0,
            ..Default::default()
        },
    ] {
        assert!(
            compile_owned_attribute_recipe(&base, &mapping, &f.catalog, &f.policy, limits).is_err()
        );
    }
    assert_eq!(base.registry().input(), &f.recipe.registry);
}
