//! Generalized input-port refinement preserves complete immutable endpoints.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::*, owned_schema::*};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits};
use poe_optimizer_import::owned_successor::*;
use serde::de::DeserializeOwned;
use std::{fs, path::PathBuf};

fn load<T: DeserializeOwned>(name: &str) -> T {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/owned/poe2/3887ae68/current")
        .join(name);
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn input() -> SuccessorBundleInput {
    SuccessorBundleInput {
        schema_version: OWNED_SUCCESSOR_VERSION,
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
                name: OwnedDefinitionKey::new(format!("original-{i:02}")).unwrap(),
                queries: load(&format!("queries-original-{i:02}.json")),
            })
            .collect(),
    }
}
fn append(input: &SuccessorBundleInput) -> CatalogAppend {
    CatalogAppend {
        mappings: vec![],
        source: input.mapping.source.clone(),
        item_policies: CatalogItemPolicyMode::RebindPrior,
    }
}
fn tree() -> TreePolicyTransitionInput {
    TreePolicyTransitionInput::RebindPrior {
        prior: Box::new(load("tree-normalization.json")),
    }
}
fn bind(input: &mut SuccessorBundleInput) {
    let schema = OwnedDefinitionSchemaPackage::new(
        input.successor.schema.clone(),
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    input.successor.rules.definitions = schema.identity().clone();
    input.successor.routing.definitions = schema.identity().clone();
}
fn close(d: &mut DeclaredSlots) {
    for c in [
        &mut d.parameters.closure,
        &mut d.choices.closure,
        &mut d.grants.closure,
        &mut d.actors.closure,
        &mut d.skill_grants.closure,
        &mut d.outputs.closure,
        &mut d.sockets.closure,
    ] {
        *c = SchemaClosure::Complete;
    }
}
fn changed() -> (SuccessorBundleInput, DeclarationClosureRefinement) {
    let mut input = input();
    let class = input
        .successor
        .schema
        .definitions
        .iter_mut()
        .find_map(|d| match d {
            DefinitionDescriptor::Class(e) => match &mut e.schema {
                SchemaState::Known(s) => {
                    close(&mut s.declarations);
                    Some(e.id.clone())
                }
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    let node = input
        .successor
        .schema
        .definitions
        .iter_mut()
        .find_map(|d| match d {
            DefinitionDescriptor::PassiveNode(e) => match &mut e.schema {
                SchemaState::Known(s) if s.declarations.choices.members.len() == 1 => {
                    close(&mut s.declarations);
                    Some(e.id.clone())
                }
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    bind(&mut input);
    let policy = DeclarationClosureRefinement {
        schema_version: 2,
        before: input.prior.rules.definitions.clone(),
        after: input.successor.rules.definitions.clone(),
        owners: vec![
            DeclarationRefinementOwner::Class(class),
            DeclarationRefinementOwner::PassiveNode(node),
        ],
    };
    (input, policy)
}
fn apply(
    input: SuccessorBundleInput,
    policy: DeclarationClosureRefinement,
) -> Result<StagedSuccessorBundle, SuccessorBundleError> {
    let append = append(&input);
    transition_owned_catalog_with_declaration_refinement(
        input,
        append,
        tree(),
        policy,
        SuccessorBundleLimits::default(),
    )
}
fn selected<'a>(
    input: &'a mut SuccessorBundleInput,
    owner: &DeclarationRefinementOwner,
) -> &'a mut DefinitionDescriptor {
    input
        .successor
        .schema
        .definitions
        .iter_mut()
        .find(|d| d.address() == owner.address())
        .unwrap()
}
fn class<'a>(
    input: &'a mut SuccessorBundleInput,
    policy: &DeclarationClosureRefinement,
) -> &'a mut ClassSchema {
    let DefinitionDescriptor::Class(e) = selected(input, &policy.owners[0]) else {
        panic!("class")
    };
    let SchemaState::Known(s) = &mut e.schema else {
        panic!("known class")
    };
    s
}

#[test]
fn mixed_owners_close_only_input_ports_and_keep_game_rule_coverage() {
    let (input, policy) = changed();
    let previous = input.clone();
    let staged = apply(input, policy.clone()).unwrap();
    assert_eq!(staged.recipe(), &previous.successor);
    assert_eq!(staged.recipe().registry, previous.prior.registry);
    assert_eq!(staged.recipe().schema.slots, previous.prior.schema.slots);
    assert_eq!(staged.recipe().rules.owners, previous.prior.rules.owners);
    assert_eq!(
        staged.recipe().rules.receivers,
        previous.prior.rules.receivers
    );
    assert_eq!(
        staged.transition().preserved_definitions,
        previous.prior.schema.definitions.len() - 2
    );
    assert_eq!(
        staged.transition().schema_policy,
        "explicit_input_declaration_closure"
    );
    assert_eq!(
        staged.transition().schema_refinement,
        Some(policy.clone().into())
    );
    assert_eq!(staged.transition().query_rows, 110);
    let metadata: SchemaDeclarationRefinement = policy.clone().into();
    metadata
        .validate_current_metadata(&policy.before, &policy.after, staged.assembled().schema())
        .unwrap();
    assert!(
        transition_owned_catalog_with_tree(
            previous.clone(),
            append(&previous),
            tree(),
            SuccessorBundleLimits::default()
        )
        .is_err()
    );
}

#[test]
fn legacy_wire_and_digest_are_exact_and_old_manifest_refinement_stays_readable() {
    let (mut input, policy) = changed();
    let old_class = input
        .prior
        .schema
        .definitions
        .iter()
        .find(|d| d.address() == policy.owners[0].address())
        .unwrap()
        .clone();
    *selected(&mut input, &policy.owners[0]) = old_class;
    bind(&mut input);
    let DeclarationRefinementOwner::PassiveNode(node) = policy.owners[1].clone() else {
        panic!("node")
    };
    let legacy = PassiveDeclarationRefinement {
        schema_version: 1,
        before: input.prior.rules.definitions.clone(),
        after: input.successor.rules.definitions.clone(),
        nodes: vec![node],
    };
    let bytes = serde_json::to_vec(&legacy).unwrap();
    let decoded: SchemaDeclarationRefinement = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(serde_json::to_vec(&decoded).unwrap(), bytes);
    let append = append(&input);
    let tree = tree();
    let digest = digest_owned(
        "owned-passive-refinement-successor-input-v1",
        &(&input, &Some(&append), &Some(&tree), &legacy),
        SuccessorBundleLimits::default().max_input_bytes,
    )
    .unwrap();
    let staged = transition_owned_catalog_with_tree_refinement(
        input,
        append,
        tree,
        legacy.clone(),
        SuccessorBundleLimits::default(),
    )
    .unwrap();
    assert_eq!(staged.transition().input, digest);
    assert_eq!(
        staged.transition().schema_policy,
        "explicit_passive_declaration_closure"
    );
    assert_eq!(
        serde_json::to_vec(staged.transition().schema_refinement.as_ref().unwrap()).unwrap(),
        bytes
    );
    decoded
        .validate_current_metadata(&legacy.before, &legacy.after, staged.assembled().schema())
        .unwrap();
}

#[test]
fn endpoints_versions_duplicate_foreign_and_absent_owners_are_rejected() {
    let (input, policy) = changed();
    let mut cases = vec![];
    let mut p = policy.clone();
    p.before = p.after.clone();
    cases.push(p);
    let mut p = policy.clone();
    p.after = p.before.clone();
    cases.push(p);
    let mut p = policy.clone();
    p.schema_version = 1;
    cases.push(p);
    let mut p = policy.clone();
    p.schema_version = 3;
    cases.push(p);
    let mut p = policy.clone();
    p.owners.clear();
    cases.push(p);
    let mut p = policy.clone();
    p.owners.push(p.owners[0].clone());
    cases.push(p);
    let mut p = policy.clone();
    p.owners[0] = DeclarationRefinementOwner::Class(
        ClassDefId::parse(GameVersionNamespace::new("foreign", "v1").unwrap(), "class").unwrap(),
    );
    cases.push(p);
    let mut p = policy.clone();
    p.owners[0] = DeclarationRefinementOwner::Class(
        ClassDefId::parse(input.prior.schema.namespace.clone(), "absent").unwrap(),
    );
    cases.push(p);
    for p in cases {
        assert!(apply(input.clone(), p).is_err());
    }
    let mut malformed = serde_json::to_value(&policy).unwrap();
    malformed["nodes"] = serde_json::json!([]);
    assert!(serde_json::from_value::<SchemaDeclarationRefinement>(malformed).is_err());
    let mut malformed = serde_json::to_value(&policy).unwrap();
    malformed["owners"][0]["kind"] = serde_json::json!("skill");
    assert!(serde_json::from_value::<SchemaDeclarationRefinement>(malformed).is_err());
}

#[test]
fn class_nonport_fields_remain_immutable_under_exact_endpoint_bindings() {
    let (input, policy) = changed();
    for field in ["minimum", "maximum", "ascendancies", "implicit_passives"] {
        let mut candidate = input.clone();
        let mut p = policy.clone();
        let s = class(&mut candidate, &p);
        match field {
            "minimum" => s.level.minimum = BoundedInteger::new(2).unwrap(),
            "maximum" => s.level.maximum = BoundedInteger::new(99).unwrap(),
            "ascendancies" => {
                assert!(!s.ascendancies.members.is_empty());
                s.ascendancies.members.clear();
            }
            "implicit_passives" => {
                assert!(!s.implicit_passives.members.is_empty());
                s.implicit_passives.members.clear();
            }
            _ => unreachable!(),
        }
        bind(&mut candidate);
        p.after = candidate.successor.rules.definitions.clone();
        assert!(
            matches!(
                apply(candidate, p),
                Err(SuccessorBundleError::Refinement(
                    "not an exact closure refinement"
                ))
            ),
            "{field}"
        );
    }
}

#[test]
fn every_port_must_close_and_existing_members_slots_ids_must_be_preserved() {
    let (input, policy) = changed();
    let prior = input
        .prior
        .schema
        .definitions
        .iter()
        .find(|d| d.address() == policy.owners[0].address())
        .unwrap();
    let DefinitionDescriptor::Class(e) = prior else {
        panic!("class")
    };
    let SchemaState::Known(prior_class) = &e.schema else {
        panic!("known")
    };
    let old = &prior_class.declarations;
    let closures = [
        &old.parameters.closure,
        &old.choices.closure,
        &old.grants.closure,
        &old.actors.closure,
        &old.skill_grants.closure,
        &old.outputs.closure,
        &old.sockets.closure,
    ];
    for (i, old) in closures.into_iter().enumerate() {
        assert!(matches!(old, SchemaClosure::Partial { .. }));
        let mut candidate = input.clone();
        let mut p = policy.clone();
        let d = &mut class(&mut candidate, &p).declarations;
        let closure = match i {
            0 => &mut d.parameters.closure,
            1 => &mut d.choices.closure,
            2 => &mut d.grants.closure,
            3 => &mut d.actors.closure,
            4 => &mut d.skill_grants.closure,
            5 => &mut d.outputs.closure,
            6 => &mut d.sockets.closure,
            _ => unreachable!(),
        };
        *closure = old.clone();
        bind(&mut candidate);
        p.after = candidate.successor.rules.definitions.clone();
        assert!(matches!(
            apply(candidate, p),
            Err(SuccessorBundleError::Refinement(
                "not an exact closure refinement"
            ))
        ));
    }
    let mut candidate = input.clone();
    let mut p = policy.clone();
    let DefinitionDescriptor::PassiveNode(e) = selected(&mut candidate, &p.owners[1]) else {
        panic!("node")
    };
    let SchemaState::Known(s) = &mut e.schema else {
        panic!("known")
    };
    let removed = s.declarations.choices.members.remove(0);
    candidate
        .successor
        .schema
        .slots
        .retain(|slot| slot.address() != SlotAddress::Choice(removed.clone()));
    bind(&mut candidate);
    p.after = candidate.successor.rules.definitions.clone();
    assert!(apply(candidate, p).is_err());
    let mut candidate = input.clone();
    let mut p = policy.clone();
    let slot = candidate
        .successor
        .schema
        .slots
        .iter_mut()
        .find_map(|s| match s {
            SlotDescriptor::Choice(e) => match &mut e.schema {
                SchemaState::Known(s) => Some(s),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    slot.presence = SlotPresence::OptionalOnce;
    bind(&mut candidate);
    p.after = candidate.successor.rules.definitions.clone();
    assert!(matches!(
        apply(candidate, p),
        Err(SuccessorBundleError::ChangedDeclaration)
    ));
    let mut candidate = input.clone();
    candidate.successor.registry.entries[0].target =
        candidate.successor.registry.entries[1].target.clone();
    assert!(apply(candidate, policy.clone()).is_err());
}

#[test]
fn no_op_replay_and_metadata_with_open_current_ports_are_rejected() {
    let (mut input, mut policy) = changed();
    let old_schema =
        OwnedDefinitionSchemaPackage::new(input.prior.schema.clone(), OwnedSchemaLimits::default())
            .unwrap();
    let metadata: SchemaDeclarationRefinement = policy.clone().into();
    assert!(
        metadata
            .validate_current_metadata(&policy.before, &policy.after, &old_schema)
            .is_err()
    );
    policy.after = policy.before.clone();
    let metadata: SchemaDeclarationRefinement = policy.clone().into();
    assert!(
        metadata
            .validate_current_metadata(&policy.before, &policy.after, &old_schema)
            .is_err()
    );
    input.successor = input.prior.clone();
    assert!(apply(input, policy).is_err());
}
