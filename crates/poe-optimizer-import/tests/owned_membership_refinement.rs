//! Endpoint-bound additions to Partial sets; no closure, scalar or rule repair.
use poe_optimizer_core::{
    owned_build::DeclaredSlot, owned_content::digest_owned, owned_definitions::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits};
use poe_optimizer_import::{owned_mapping::OwnedIdRegistry, owned_recipe::*, owned_successor::*};
use serde::de::DeserializeOwned;
use std::{fs, path::PathBuf};

fn key(v: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(v).unwrap()
}
fn load<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(
        &fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../data/owned/poe2/3887ae68/current")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
fn input() -> SuccessorBundleInput {
    SuccessorBundleInput {
        schema_version: 1,
        prior: load("recipe.json"),
        successor: load("recipe.json"),
        mapping: load("mapping.json"),
        roles: load("roles.json"),
        normalization: load("normalization.json"),
        rewards: load("rewards.json"),
        items: load("items.json"),
        item_source: load("item-source.json"),
        query_sets: (1..=5)
            .map(|i| NamedQuerySet {
                name: key(&format!("original-{i:02}")),
                queries: load(&format!("queries-original-{i:02}.json")),
            })
            .collect(),
    }
}
fn bind(recipe: &mut OwnedRecipeInput) {
    let schema =
        OwnedDefinitionSchemaPackage::new(recipe.schema.clone(), OwnedSchemaLimits::default())
            .unwrap();
    recipe.rules.definitions = schema.identity().clone();
    recipe.routing.definitions = schema.identity().clone();
}
fn staged(recipe: OwnedRecipeInput) -> StagedOwnedRecipe {
    assemble_owned_recipe(recipe, Default::default()).unwrap()
}
fn ports() -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(vec![]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}
fn item(recipe: &mut OwnedRecipeInput) -> &mut ItemTemplateSchema {
    recipe
        .schema
        .definitions
        .iter_mut()
        .find_map(|d| match d {
            DefinitionDescriptor::ItemTemplate(e) => match &mut e.schema {
                SchemaState::Known(s) => Some(s),
                _ => None,
            },
            _ => None,
        })
        .unwrap()
}
fn partial(subject: SchemaSubject) -> SchemaClosure {
    SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject,
            facet: SchemaFacet::InputSchema,
            code: key("unconverted-fixture-inputs"),
        }],
    }
}
fn changed() -> (SuccessorBundleInput, SchemaMembershipRefinement) {
    let mut input = input();
    let mut registry =
        OwnedIdRegistry::new(input.successor.registry.clone(), Default::default()).unwrap();
    let modifier: ModifierDefId = registry.allocate_definition().unwrap();
    let quality: QualityDefId = registry.allocate_definition().unwrap();
    let quality_schema = input
        .successor
        .schema
        .definitions
        .iter()
        .find_map(|d| match d {
            DefinitionDescriptor::Quality(e) => match &e.schema {
                SchemaState::Known(s) => Some(s.clone()),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    input
        .successor
        .schema
        .definitions
        .push(DefinitionDescriptor::Modifier(DefinitionEntry {
            id: modifier.clone(),
            schema: SchemaState::Known(ModifierSchema {
                declarations: ports(),
            }),
        }));
    input
        .successor
        .schema
        .definitions
        .push(DefinitionDescriptor::Quality(DefinitionEntry {
            id: quality.clone(),
            schema: SchemaState::Known(quality_schema),
        }));
    input.successor.registry = registry.input().clone();
    let item_id = input
        .successor
        .schema
        .definitions
        .iter()
        .find_map(|d| match d {
            DefinitionDescriptor::ItemTemplate(e) => Some(e.id.clone()),
            _ => None,
        })
        .unwrap();
    item(&mut input.successor).modifiers.members.push(modifier);
    item(&mut input.successor)
        .quality
        .allowed_kinds
        .members
        .push(quality);
    bind(&mut input.successor);
    let policy = SchemaMembershipRefinement {
        schema_version: 3,
        before: input.prior.rules.definitions.clone(),
        after: input.successor.rules.definitions.clone(),
        subjects: vec![SchemaSubject::Definition(item_id.address())],
    };
    (input, policy)
}
fn update_policy(input: &mut SuccessorBundleInput, policy: &mut SchemaMembershipRefinement) {
    bind(&mut input.prior);
    bind(&mut input.successor);
    policy.before = input.prior.rules.definitions.clone();
    policy.after = input.successor.rules.definitions.clone();
}
fn validate(
    input: &SuccessorBundleInput,
    policy: &SchemaMembershipRefinement,
) -> Result<(), SuccessorBundleError> {
    validate_schema_membership_refinement(
        policy,
        &staged(input.prior.clone()),
        &staged(input.successor.clone()),
    )
}
fn apply(
    input: SuccessorBundleInput,
    policy: SchemaMembershipRefinement,
    compact: bool,
) -> Result<StagedSuccessorBundle, SuccessorBundleError> {
    let append = CatalogAppend {
        mappings: vec![],
        source: input.mapping.source.clone(),
        item_policies: CatalogItemPolicyMode::RebindPrior,
    };
    let tree = TreePolicyTransitionInput::RebindPrior {
        prior: Box::new(load("tree-normalization.json")),
    };
    if compact {
        transition_owned_catalog_with_membership_refinement_compact(
            input,
            append,
            tree,
            policy,
            Default::default(),
        )
    } else {
        transition_owned_catalog_with_membership_refinement(
            input,
            append,
            tree,
            policy,
            Default::default(),
        )
    }
}

#[test]
fn modifiers_and_quality_are_added_without_closure_or_other_field_changes() {
    let (input, policy) = changed();
    validate(&input, &policy).unwrap();
    let before = input.clone();
    let result = apply(input, policy.clone(), false).unwrap();
    assert_eq!(result.recipe(), &before.successor);
    assert_eq!(
        result.transition().schema_policy,
        "explicit_partial_schema_membership"
    );
    assert_eq!(
        result.transition().preserved_definitions,
        before.prior.schema.definitions.len() - 1
    );
    assert_eq!(
        result.transition().preserved_slots,
        before.prior.schema.slots.len()
    );
    assert_eq!(result.recipe().rules.owners, before.prior.rules.owners);
    let metadata: SchemaDeclarationRefinement = policy.clone().into();
    assert_eq!(
        result.transition().schema_refinement,
        Some(metadata.clone())
    );
    metadata
        .validate_current_metadata(&policy.before, &policy.after, result.assembled().schema())
        .unwrap();
    assert!(transition_owned_bundle(before, Default::default()).is_err());
}

#[test]
fn compact_and_legacy_membership_publication_bind_exact_inputs_and_preserve_existing_formats() {
    let (input, policy) = changed();
    let append = CatalogAppend {
        mappings: vec![],
        source: input.mapping.source.clone(),
        item_policies: CatalogItemPolicyMode::RebindPrior,
    };
    let tree = TreePolicyTransitionInput::RebindPrior {
        prior: Box::new(load("tree-normalization.json")),
    };
    let expected = digest_owned(
        "owned-schema-membership-successor-input-v3",
        &(&input, &Some(append), &Some(tree), &policy),
        64 * 1024 * 1024,
    )
    .unwrap();
    let old = apply(input.clone(), policy.clone(), false).unwrap();
    let compact = apply(input, policy.clone(), true).unwrap();
    assert_eq!(old.transition().input, expected);
    assert_eq!(old.transition().schema_version, 1);
    assert_eq!(compact.transition().schema_version, 2);
    assert_eq!(old.transition().before, compact.transition().before);
    assert_eq!(old.transition().after, compact.transition().after);
    assert_ne!(old.transition().input, compact.transition().input);
    assert!(old.artifacts().any(|(name, _)| name == "recipe.json"));
    assert!(!compact.artifacts().any(|(name, _)| name == "recipe.json"));
    let bytes = serde_json::to_vec(&SchemaDeclarationRefinement::from(policy.clone())).unwrap();
    assert_eq!(bytes, serde_json::to_vec(&policy).unwrap());
    let mut mixed = serde_json::to_value(&policy).unwrap();
    mixed["owners"] = serde_json::json!([]);
    assert!(serde_json::from_value::<SchemaDeclarationRefinement>(mixed).is_err());
}

#[test]
fn closure_changes_complete_growth_member_removal_and_scalar_edits_are_rejected() {
    let (input, policy) = changed();
    for case in 0..6 {
        let mut candidate = input.clone();
        let mut p = policy.clone();
        match case {
            0 => item(&mut candidate.successor).modifiers.closure = SchemaClosure::Complete,
            1 => {
                item(&mut candidate.prior).modifiers.closure = SchemaClosure::Complete;
                item(&mut candidate.successor).modifiers.closure = SchemaClosure::Complete;
            }
            2 => {
                item(&mut candidate.successor).modifiers.members.remove(0);
            }
            3 => {
                item(&mut candidate.successor).item_level.maximum = BoundedInteger::new(99).unwrap()
            }
            4 => item(&mut candidate.successor).quality.presence = QualityPresence::Required,
            _ => {
                let SchemaClosure::Partial { gaps } =
                    &mut item(&mut candidate.successor).modifiers.closure
                else {
                    panic!()
                };
                gaps[0].code = key("replacement-gap");
            }
        }
        update_policy(&mut candidate, &mut p);
        assert!(validate(&candidate, &p).is_err(), "case {case}");
    }
}

#[test]
fn explicit_subjects_endpoints_and_known_state_are_required() {
    let (input, policy) = changed();
    for case in 0..7 {
        let mut p = policy.clone();
        match case {
            0 => p.schema_version = 2,
            1 => p.before = p.after.clone(),
            2 => p.after = p.before.clone(),
            3 => p.subjects.clear(),
            4 => p.subjects.push(p.subjects[0].clone()),
            5 => {
                p.subjects[0] = SchemaSubject::Definition(
                    ItemTemplateDefId::parse(
                        GameVersionNamespace::new("foreign", "v1").unwrap(),
                        "other",
                    )
                    .unwrap()
                    .address(),
                )
            }
            _ => {
                p.subjects[0] = SchemaSubject::Definition(
                    ItemTemplateDefId::parse(input.prior.schema.namespace.clone(), "missing")
                        .unwrap()
                        .address(),
                )
            }
        }
        assert!(validate(&input, &p).is_err(), "case {case}");
    }
    let mut p = policy.clone();
    p.subjects.push(
        input
            .prior
            .schema
            .definitions
            .iter()
            .find_map(|d| match d {
                DefinitionDescriptor::Class(e) => Some(SchemaSubject::Definition(e.id.address())),
                _ => None,
            })
            .unwrap(),
    );
    assert!(validate(&input, &p).is_err()); // Each listed subject needs actual additions.
    // Existing item owners have compiled programs and cannot be staged as
    // Unmapped. Use an independently allocated, unused descriptor so this
    // assertion reaches the refinement boundary instead of rule compilation.
    let mut before = input.prior.clone();
    let mut registry = OwnedIdRegistry::new(before.registry.clone(), Default::default()).unwrap();
    let id: ModifierDefId = registry.allocate_definition().unwrap();
    let subject = SchemaSubject::Definition(id.address());
    before.registry = registry.input().clone();
    before
        .schema
        .definitions
        .push(DefinitionDescriptor::Modifier(DefinitionEntry {
            id: id.clone(),
            schema: SchemaState::Unmapped {
                gaps: vec![SchemaGap {
                    subject: subject.clone(),
                    facet: SchemaFacet::InputSchema,
                    code: key("unmapped-test"),
                }],
            },
        }));
    bind(&mut before);
    let mut after = before.clone();
    let row = after
        .schema
        .definitions
        .iter_mut()
        .find(|row| row.address() == id.address())
        .unwrap();
    *row = DefinitionDescriptor::Modifier(DefinitionEntry {
        id,
        schema: SchemaState::Known(ModifierSchema {
            declarations: ports(),
        }),
    });
    bind(&mut after);
    let p = SchemaMembershipRefinement {
        schema_version: 3,
        before: before.rules.definitions.clone(),
        after: after.rules.definitions.clone(),
        subjects: vec![subject],
    };
    assert!(validate_schema_membership_refinement(&p, &staged(before), &staged(after)).is_err());
}

#[test]
fn undeclared_descriptor_edits_and_registry_history_rewrites_remain_forbidden() {
    let (mut input, mut policy) = changed();
    let original = input.clone();
    let row = input
        .successor
        .schema
        .definitions
        .iter_mut()
        .find_map(|d| match d {
            DefinitionDescriptor::Class(e) => match &mut e.schema {
                SchemaState::Known(s) => Some(s),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    row.level.maximum = BoundedInteger::new(99).unwrap();
    update_policy(&mut input, &mut policy);
    assert!(validate(&input, &policy).is_err());
    let mut input = original;
    input.successor.registry.entries.remove(0);
    assert!(assemble_owned_recipe(input.successor, Default::default()).is_err());
}

#[test]
fn generated_skill_actor_and_action_output_memberships_refine_independently() {
    let mut before: OwnedRecipeInput = load("recipe.json");
    let skill = before
        .schema
        .definitions
        .iter()
        .find_map(|d| match d {
            DefinitionDescriptor::Skill(e) => match &e.schema {
                SchemaState::Known(_) => Some(e.id.clone()),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    let actor = before
        .schema
        .slots
        .iter()
        .find_map(|d| match d {
            SlotDescriptor::Actor(e) => Some(e.id.clone()),
            _ => None,
        })
        .unwrap();
    let output = before
        .schema
        .slots
        .iter()
        .find_map(|d| match d {
            SlotDescriptor::ActionOutput(e) => Some(e.id.clone()),
            _ => None,
        })
        .unwrap();
    let new_skill = before
        .schema
        .definitions
        .iter()
        .find_map(|d| match d {
            DefinitionDescriptor::Skill(e)
                if matches!(e.schema, SchemaState::Known(_))
                    && !matches!(&actor.declaration,SlotOwnerDefId::Skill(v) if v==&e.id) =>
            {
                Some(e.id.clone())
            }
            _ => None,
        })
        .unwrap();
    for row in &mut before.schema.definitions {
        if let DefinitionDescriptor::Skill(e) = row
            && e.id == skill
        {
            let SchemaState::Known(s) = &mut e.schema else {
                panic!()
            };
            s.declarations.parameters.closure = partial(SchemaSubject::Definition(skill.address()));
        }
    }
    for row in &mut before.schema.slots {
        match row {
            SlotDescriptor::Actor(e) if e.id == actor => {
                let SchemaState::Known(s) = &mut e.schema else {
                    panic!()
                };
                s.skills.closure = partial(SchemaSubject::Slot(SlotAddress::Actor(actor.clone())));
            }
            SlotDescriptor::ActionOutput(e) if e.id == output => {
                let SchemaState::Known(s) = &mut e.schema else {
                    panic!()
                };
                s.parts.closure = partial(SchemaSubject::Slot(SlotAddress::ActionOutput(
                    output.clone(),
                )));
            }
            _ => {}
        }
    }
    bind(&mut before);
    let mut after = before.clone();
    let mut registry = OwnedIdRegistry::new(after.registry.clone(), Default::default()).unwrap();
    let parameter: DeclaredSlot<ParameterSlotDefId> = registry
        .allocate_slot(SlotOwnerDefId::Skill(skill.clone()))
        .unwrap();
    let part: ActionPartDefId = registry.allocate_definition().unwrap();
    after.registry = registry.input().clone();
    after
        .schema
        .definitions
        .push(DefinitionDescriptor::ActionPart(DefinitionEntry {
            id: part.clone(),
            schema: SchemaState::Known(ActionPartSchema {}),
        }));
    after
        .schema
        .slots
        .push(SlotDescriptor::Parameter(DefinitionEntry {
            id: parameter.clone(),
            schema: SchemaState::Known(ParameterSlotSchema {
                value: ValueSchema::Boolean,
                presence: SlotPresence::OptionalOnce,
                sites: vec![],
            }),
        }));
    for row in &mut after.schema.definitions {
        if let DefinitionDescriptor::Skill(e) = row
            && e.id == skill
        {
            let SchemaState::Known(s) = &mut e.schema else {
                panic!()
            };
            s.declarations.parameters.members.push(parameter.clone());
        }
    }
    for row in &mut after.schema.slots {
        match row {
            SlotDescriptor::Actor(e) if e.id == actor => {
                let SchemaState::Known(s) = &mut e.schema else {
                    panic!()
                };
                assert!(!s.skills.members.contains(&new_skill));
                s.skills.members.push(new_skill.clone());
            }
            SlotDescriptor::ActionOutput(e) if e.id == output => {
                let SchemaState::Known(s) = &mut e.schema else {
                    panic!()
                };
                s.parts.members.push(part.clone());
            }
            _ => {}
        }
    }
    bind(&mut after);
    let policy = SchemaMembershipRefinement {
        schema_version: 3,
        before: before.rules.definitions.clone(),
        after: after.rules.definitions.clone(),
        subjects: vec![
            SchemaSubject::Definition(skill.address()),
            SchemaSubject::Slot(SlotAddress::Actor(actor)),
            SchemaSubject::Slot(SlotAddress::ActionOutput(output)),
        ],
    };
    let before = staged(before);
    let after = staged(after);
    validate_schema_membership_refinement(&policy, &before, &after).unwrap();
    SchemaDeclarationRefinement::from(policy.clone())
        .validate_current_metadata(&policy.before, &policy.after, after.schema())
        .unwrap();
    for i in 0..policy.subjects.len() {
        let mut p = policy.clone();
        p.subjects.remove(i);
        assert!(validate_schema_membership_refinement(&p, &before, &after).is_err());
    }
}
