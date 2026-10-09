//! Ascendancy uses the existing exact declaration-only publication boundary.
use super::*;
use std::collections::BTreeMap;

fn changed() -> (SuccessorBundleInput, DeclarationClosureRefinement) {
    let mut input = input();
    let ascendancy = input
        .successor
        .schema
        .definitions
        .iter_mut()
        .find_map(|d| match d {
            DefinitionDescriptor::Ascendancy(e) => match &mut e.schema {
                SchemaState::Known(s) => {
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
        owners: vec![DeclarationRefinementOwner::Ascendancy(ascendancy)],
    };
    (input, policy)
}

fn schema<'a>(
    input: &'a mut SuccessorBundleInput,
    policy: &DeclarationClosureRefinement,
) -> &'a mut AscendancySchema {
    let DefinitionDescriptor::Ascendancy(e) = selected(input, &policy.owners[0]) else {
        panic!("ascendancy")
    };
    let SchemaState::Known(s) = &mut e.schema else {
        panic!("known ascendancy")
    };
    s
}

fn apply(
    input: SuccessorBundleInput,
    policy: DeclarationClosureRefinement,
) -> Result<StagedSuccessorBundle, SuccessorBundleError> {
    let append = append(&input);
    transition_owned_catalog_with_declaration_refinement_compact(
        input,
        append,
        tree(),
        policy,
        SuccessorBundleLimits::default(),
    )
}

#[test]
fn compact_ascendancy_closure_preserves_all_nonport_data_and_rebuilds_exactly() {
    let (input, policy) = changed();
    let prior = input.clone();
    let staged = apply(input, policy.clone()).unwrap();
    assert_eq!(staged.transition().schema_version, 2);
    assert_eq!(
        staged.transition().preserved_definitions,
        prior.prior.schema.definitions.len() - 1
    );
    assert_eq!(staged.transition().query_rows, 110);
    assert_eq!(staged.recipe().registry, prior.prior.registry);
    assert_eq!(staged.recipe().schema.slots, prior.prior.schema.slots);
    assert_eq!(staged.recipe().rules.owners, prior.prior.rules.owners);
    assert_eq!(staged.recipe().rules.receivers, prior.prior.rules.receivers);
    let metadata: SchemaDeclarationRefinement = policy.clone().into();
    metadata
        .validate_current_metadata(&policy.before, &policy.after, staged.assembled().schema())
        .unwrap();
    let encoded = serde_json::to_vec(&metadata).unwrap();
    assert_eq!(
        serde_json::from_slice::<SchemaDeclarationRefinement>(&encoded).unwrap(),
        metadata
    );
    let DefinitionDescriptor::Ascendancy(old) = prior
        .prior
        .schema
        .definitions
        .iter()
        .find(|d| d.address() == policy.owners[0].address())
        .unwrap()
    else {
        panic!("previous ascendancy")
    };
    let mut restored = staged.recipe().schema.clone();
    let DefinitionDescriptor::Ascendancy(new) = restored
        .definitions
        .iter_mut()
        .find(|d| d.address() == policy.owners[0].address())
        .unwrap()
    else {
        panic!("published ascendancy")
    };
    let (SchemaState::Known(old), SchemaState::Known(new)) = (&old.schema, &mut new.schema) else {
        panic!("known endpoints")
    };
    new.declarations = old.declarations.clone();
    // Compaction canonicalizes definition storage; compare exact descriptors by ID.
    let descriptors = |s: &poe_optimizer_data::owned_schema::SchemaPackageInput| {
        s.definitions
            .iter()
            .map(|d| (d.address(), d.clone()))
            .collect::<BTreeMap<_, _>>()
    };
    assert_eq!(descriptors(&restored), descriptors(&prior.prior.schema));
    let second = apply(prior.clone(), policy).unwrap();
    assert_eq!(
        serde_json::to_value(staged.transition()).unwrap(),
        serde_json::to_value(second.transition()).unwrap()
    );
    assert_eq!(
        staged.artifacts().collect::<BTreeMap<_, _>>(),
        second.artifacts().collect::<BTreeMap<_, _>>()
    );
    assert!(
        transition_owned_catalog_with_tree_compact(
            prior.clone(),
            append(&prior),
            tree(),
            Default::default()
        )
        .is_err(),
        "ordinary publication cannot change declarations"
    );
}

#[test]
fn ascendancy_class_and_root_membership_cannot_change_with_the_closure() {
    let (input, policy) = changed();
    for field in ["classes", "implicit_passives"] {
        let mut candidate = input.clone();
        let mut p = policy.clone();
        let s = schema(&mut candidate, &p);
        match field {
            "classes" => {
                assert!(!s.classes.members.is_empty());
                s.classes.members.clear();
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
fn all_ascendancy_ports_must_close_and_current_metadata_cannot_hide_partial_ports() {
    let (input, policy) = changed();
    for field in 0..7 {
        let mut candidate = input.clone();
        let mut p = policy.clone();
        let gap = SchemaClosure::Partial {
            gaps: vec![SchemaGap {
                subject: SchemaSubject::Definition(policy.owners[0].address()),
                facet: SchemaFacet::InputSchema,
                code: OwnedDefinitionKey::new("still-unreviewed").unwrap(),
            }],
        };
        let d = &mut schema(&mut candidate, &p).declarations;
        let closure = match field {
            0 => &mut d.parameters.closure,
            1 => &mut d.choices.closure,
            2 => &mut d.grants.closure,
            3 => &mut d.actors.closure,
            4 => &mut d.skill_grants.closure,
            5 => &mut d.outputs.closure,
            6 => &mut d.sockets.closure,
            _ => unreachable!(),
        };
        *closure = gap;
        bind(&mut candidate);
        p.after = candidate.successor.rules.definitions.clone();
        let endpoint = OwnedDefinitionSchemaPackage::new(
            candidate.successor.schema.clone(),
            Default::default(),
        )
        .unwrap();
        let metadata: SchemaDeclarationRefinement = p.clone().into();
        assert!(matches!(
            metadata.validate_current_metadata(&p.before, &p.after, &endpoint),
            Err(SuccessorBundleError::Refinement(
                "current owner ports are incomplete"
            ))
        ));
        assert!(matches!(
            apply(candidate, p),
            Err(SuccessorBundleError::Refinement(
                "not an exact closure refinement"
            ))
        ));
    }
}

#[test]
fn ascendancy_policy_requires_exact_typed_unique_existing_owners_and_endpoints() {
    let (input, policy) = changed();
    let mut cases = vec![];
    let mut p = policy.clone();
    p.owners.push(p.owners[0].clone());
    cases.push(p);
    for ns in [
        input.prior.schema.namespace.clone(),
        GameVersionNamespace::new("foreign", "v1").unwrap(),
    ] {
        let mut p = policy.clone();
        p.owners[0] =
            DeclarationRefinementOwner::Ascendancy(AscendancyDefId::parse(ns, "absent").unwrap());
        cases.push(p);
    }
    let mut p = policy.clone();
    p.before = p.after.clone();
    cases.push(p);
    let mut p = policy.clone();
    p.after = p.before.clone();
    cases.push(p);
    for p in cases {
        assert!(apply(input.clone(), p).is_err());
    }
    let mut malformed = serde_json::to_value(&policy).unwrap();
    malformed["owners"][0]["kind"] = serde_json::json!("class");
    assert!(serde_json::from_value::<SchemaDeclarationRefinement>(malformed).is_err());
    let mut no_op = input;
    no_op.successor = no_op.prior.clone();
    let mut p = policy;
    p.after = p.before.clone();
    assert!(apply(no_op, p).is_err());
}
