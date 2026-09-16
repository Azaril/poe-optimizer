//! Source-independent route storage/type/closure laws. No runtime occurrence resolution.
use poe_optimizer_core::{
    owned_build::DeclaredSlot, owned_definitions::*, owned_routing::*, owned_schema::*,
};
use poe_optimizer_data::{owned_routing::*, owned_schema::*};
use serde_json::json;
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("routing-test", "v1").unwrap()
}
fn id<K: DefinitionDomain>(s: &str) -> DefId<K> {
    DefId::new(ns(), key(s))
}
fn output(s: &str) -> DeclaredSlot<ActionOutputDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(id("skill")),
        slot: id(s),
    }
}
fn known<I, D>(id: I, schema: D) -> DefinitionEntry<I, D> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn declarations() -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(vec![]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![output("first"), output("second")]),
    }
}
fn schema_input() -> SchemaPackageInput {
    let mut definitions = vec![
        DefinitionDescriptor::Skill(known(
            id("skill"),
            SkillSchema {
                directly_selectable: true,
                declarations: declarations(),
            },
        )),
        DefinitionDescriptor::Unit(known(
            id("damage"),
            UnitSchema {
                dimension: UnitDimension::Damage,
            },
        )),
        DefinitionDescriptor::Unit(known(
            id("other-damage"),
            UnitSchema {
                dimension: UnitDimension::Damage,
            },
        )),
        DefinitionDescriptor::EquipmentSlot(known(
            id("weapon"),
            EquipmentSlotSchema {
                scope: ScopePolicy::Either,
            },
        )),
        DefinitionDescriptor::ActionPart(known(id("part"), ActionPartSchema {})),
        DefinitionDescriptor::ActionPart(known(id("other-part"), ActionPartSchema {})),
        DefinitionDescriptor::ActionMode(known(id("mode"), ActionModeSchema {})),
        DefinitionDescriptor::ActionStatSet(known(id("stats"), ActionStatSetSchema {})),
    ];
    for (name, kind, unit) in [
        ("equipment", RuleEntityKind::EquipmentUse, "damage"),
        ("actor", RuleEntityKind::Actor, "damage"),
        ("action", RuleEntityKind::Action, "damage"),
        ("action-other-unit", RuleEntityKind::Action, "other-damage"),
    ] {
        definitions.push(DefinitionDescriptor::Stat(known(
            id(name),
            StatSchema {
                value: ComputedValueType::Quantity { unit: id(unit) },
                targets: vec![kind],
            },
        )));
    }
    definitions.push(DefinitionDescriptor::Stat(known(
        id("action-integer"),
        StatSchema {
            value: ComputedValueType::Integer,
            targets: vec![RuleEntityKind::Action],
        },
    )));
    let slots = ["first", "second"]
        .into_iter()
        .map(|name| {
            SlotDescriptor::ActionOutput(known(
                output(name),
                ActionOutputSchema {
                    actor_role: DeclaredActorRole::ProviderActor,
                    parts: DeclaredSet::complete(vec![id("part")]),
                    modes: DeclaredSet::complete(vec![id("mode")]),
                    stat_sets: DeclaredSet::complete(vec![id("stats")]),
                    choices: DeclaredSet::complete(vec![]),
                },
            ))
        })
        .collect();
    SchemaPackageInput {
        schema_version: poe_optimizer_data::owned_schema::OWNED_SCHEMA_PACKAGE_VERSION,
        namespace: ns(),
        release: key("schema"),
        semantics_version: key("v1"),
        definitions,
        slots,
    }
}
fn schema() -> OwnedDefinitionSchemaPackage {
    OwnedDefinitionSchemaPackage::new(schema_input(), OwnedSchemaLimits::default()).unwrap()
}
fn equipment() -> ActionStatRoute {
    ActionStatRoute {
        id: key("equipment"),
        selection: ActionRouteSelection::All,
        source: ActionStatRouteSource::PlayerEquipment {
            slot: id("weapon"),
            stat: id("equipment"),
        },
        target: id("action"),
    }
}
fn actor() -> ActionStatRoute {
    ActionStatRoute {
        id: key("actor"),
        selection: ActionRouteSelection::Exact(Box::new(ActionRouteSelector {
            part: id("part"),
            mode: id("mode"),
            stat_set: id("stats"),
        })),
        source: ActionStatRouteSource::ActionActor { stat: id("actor") },
        target: id("action"),
    }
}
fn input(s: &OwnedDefinitionSchemaPackage) -> ActionRoutingInput {
    ActionRoutingInput {
        schema_version: OWNED_ACTION_ROUTING_VERSION,
        namespace: ns(),
        release: key("routes"),
        definitions: s.identity().clone(),
        outputs: vec![ActionOutputRoutes {
            source_selectors: Some(DeclaredSet::complete(vec![])),
            output: output("first"),
            routes: DeclaredSet::complete(vec![equipment(), actor()]),
        }],
    }
}
fn package(i: ActionRoutingInput, s: &OwnedDefinitionSchemaPackage) -> OwnedActionRouting {
    OwnedActionRouting::new(i, s, RoutingLimits::default()).unwrap()
}
fn gap(out: &str, code: &str) -> SchemaGap {
    SchemaGap {
        subject: SchemaSubject::Slot(SlotAddress::ActionOutput(output(out))),
        facet: SchemaFacet::GameRules,
        code: key(code),
    }
}
#[test]
fn canonical_roundtrip_preserves_overlapping_routes_without_choosing_a_winner() {
    let s = schema();
    let mut i = input(&s);
    i.outputs.push(ActionOutputRoutes {
        source_selectors: Some(DeclaredSet::complete(vec![])),
        output: output("second"),
        routes: DeclaredSet::partial(vec![actor()], vec![gap("second", "z"), gap("second", "a")]),
    });
    let p = package(i.clone(), &s);
    i.outputs.reverse();
    for o in &mut i.outputs {
        o.routes.members.reverse();
        if let SchemaClosure::Partial { gaps } = &mut o.routes.closure {
            gaps.reverse();
        }
    }
    let permuted = package(i, &s);
    assert_eq!(p.identity(), permuted.identity());
    assert_eq!(p.input(), permuted.input());
    let bytes = encode_action_routing(&p, RoutingLimits::default()).unwrap();
    let decoded = decode_action_routing(&bytes, &s, RoutingLimits::default()).unwrap();
    assert_eq!(p.identity(), decoded.identity());
    assert_eq!(p.input(), decoded.input());
    let selected = p.routes_for(&output("first")).unwrap();
    assert_eq!(selected.members.len(), 2);
    assert_eq!(selected.members[0].target, selected.members[1].target);
    assert_eq!(p.resources().outputs, 2);
    assert_eq!(p.resources().routes, 3);
    assert_eq!(p.resources().gaps, 2);
    assert!(!p.routes_for(&output("second")).unwrap().is_complete());
    assert!(
        serde_json::from_slice::<serde_json::Value>(&bytes)
            .unwrap()
            .get("source")
            .is_none()
    );
}
#[test]
fn absent_output_complete_empty_and_partial_empty_have_distinct_content() {
    let s = schema();
    let mut i = input(&s);
    i.outputs.clear();
    let absent = package(i.clone(), &s);
    assert!(absent.routes_for(&output("first")).is_none());
    i.outputs.push(ActionOutputRoutes {
        source_selectors: Some(DeclaredSet::complete(vec![])),
        output: output("first"),
        routes: DeclaredSet::complete(vec![]),
    });
    let empty = package(i.clone(), &s);
    assert!(empty.routes_for(&output("first")).unwrap().is_complete());
    i.outputs[0].routes.closure = SchemaClosure::Partial {
        gaps: vec![gap("first", "unconverted")],
    };
    let partial = package(i, &s);
    assert!(!partial.routes_for(&output("first")).unwrap().is_complete());
    assert_ne!(absent.identity(), empty.identity());
    assert_ne!(empty.identity(), partial.identity());
}
#[test]
fn exact_units_and_source_target_kinds_are_required() {
    let s = schema();
    for target in ["action-other-unit", "action-integer", "actor"] {
        let mut i = input(&s);
        i.outputs[0].routes.members[0].target = id(target);
        assert!(
            OwnedActionRouting::new(i, &s, RoutingLimits::default()).is_err(),
            "{target}"
        );
    }
    let mut i = input(&s);
    i.outputs[0].routes.members[0].source = ActionStatRouteSource::PlayerEquipment {
        slot: id("weapon"),
        stat: id("actor"),
    };
    assert!(
        OwnedActionRouting::new(i, &s, RoutingLimits::default())
            .unwrap_err()
            .to_string()
            .contains("target kind")
    );
    let mut i = input(&s);
    i.outputs[0].routes.members[0].source = ActionStatRouteSource::ActionActor {
        stat: id("equipment"),
    };
    assert!(OwnedActionRouting::new(i, &s, RoutingLimits::default()).is_err());
}
#[test]
fn missing_or_foreign_sources_and_exact_output_owners_reject() {
    let s = schema();
    let mut i = input(&s);
    i.outputs[0].routes.members[0].source = ActionStatRouteSource::PlayerEquipment {
        slot: id("missing"),
        stat: id("equipment"),
    };
    assert!(
        OwnedActionRouting::new(i, &s, RoutingLimits::default())
            .unwrap_err()
            .to_string()
            .contains("missing schema")
    );
    let mut i = input(&s);
    i.outputs[0].output.declaration = SlotOwnerDefId::Skill(id("another-owner"));
    assert!(OwnedActionRouting::new(i, &s, RoutingLimits::default()).is_err());
    let mut i = input(&s);
    i.outputs[0].routes.members[0].target = DefId::new(
        GameVersionNamespace::new("foreign", "v1").unwrap(),
        key("action"),
    );
    assert!(
        OwnedActionRouting::new(i, &s, RoutingLimits::default())
            .unwrap_err()
            .to_string()
            .contains("foreign namespace")
    );
}
#[test]
fn exact_selector_membership_is_required_and_listed_partial_members_remain_usable() {
    let s = schema();
    let mut i = input(&s);
    let ActionRouteSelection::Exact(selector) = &mut i.outputs[0].routes.members[1].selection
    else {
        panic!()
    };
    selector.part = id("other-part");
    assert!(
        OwnedActionRouting::new(i, &s, RoutingLimits::default())
            .unwrap_err()
            .to_string()
            .contains("declared member")
    );
    let mut raw = schema_input();
    let SlotDescriptor::ActionOutput(entry) = &mut raw.slots[0] else {
        panic!()
    };
    let SchemaState::Known(output_schema) = &mut entry.schema else {
        panic!()
    };
    output_schema.parts.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            facet: SchemaFacet::StaticLinks,
            ..gap("first", "unknown-parts")
        }],
    };
    let partial = OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap();
    let p = package(input(&partial), &partial);
    assert_eq!(p.resources().routes, 2);
}
#[test]
fn unmapped_stat_schema_never_becomes_a_route() {
    let mut raw = schema_input();
    for descriptor in &mut raw.definitions {
        if let DefinitionDescriptor::Stat(entry) = descriptor
            && entry.id == id("equipment")
        {
            entry.schema = SchemaState::Unmapped {
                gaps: vec![SchemaGap {
                    subject: SchemaSubject::Definition(DefinitionAddress::Stat(entry.id.clone())),
                    facet: SchemaFacet::GameRules,
                    code: key("not-converted"),
                }],
            };
        }
    }
    let s = OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap();
    assert!(
        OwnedActionRouting::new(input(&s), &s, RoutingLimits::default())
            .unwrap_err()
            .to_string()
            .contains("unmapped schema")
    );
}
#[test]
fn duplicate_ids_outputs_and_malformed_closure_gaps_reject() {
    let s = schema();
    let mut i = input(&s);
    i.outputs.push(i.outputs[0].clone());
    assert!(
        OwnedActionRouting::new(i, &s, RoutingLimits::default())
            .unwrap_err()
            .to_string()
            .contains("duplicate output")
    );
    let mut i = input(&s);
    i.outputs[0].routes.members.push(equipment());
    assert!(
        OwnedActionRouting::new(i, &s, RoutingLimits::default())
            .unwrap_err()
            .to_string()
            .contains("duplicate route")
    );
    for gaps in [
        vec![],
        vec![gap("second", "wrong-owner")],
        vec![gap("first", "same"), gap("first", "same")],
        vec![SchemaGap {
            facet: SchemaFacet::StaticLinks,
            ..gap("first", "wrong-facet")
        }],
    ] {
        let mut i = input(&s);
        i.outputs[0].routes.closure = SchemaClosure::Partial { gaps };
        assert!(OwnedActionRouting::new(i, &s, RoutingLimits::default()).is_err());
    }
}
#[test]
fn strict_codec_rejects_unknown_duplicate_missing_fields_and_wrong_id_domains() {
    let s = schema();
    let bytes = encode_action_routing(&package(input(&s), &s), RoutingLimits::default()).unwrap();
    let base: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let mut unknown = base.clone();
    unknown["unexpected"] = json!(true);
    let mut missing = base.clone();
    missing.as_object_mut().unwrap().remove("outputs");
    let mut no_selection = base.clone();
    no_selection["outputs"][0]["routes"]["members"][0]
        .as_object_mut()
        .unwrap()
        .remove("selection");
    let mut extra_source = base.clone();
    extra_source["outputs"][0]["routes"]["members"][0]["source"]["value"]["extra"] = json!(true);
    let mut wrong_kind = base.clone();
    wrong_kind["outputs"][0]["routes"]["members"][0]["target"]["kind"] = json!("gem");
    for value in [unknown, missing, no_selection, extra_source, wrong_kind] {
        assert!(
            decode_action_routing(
                &serde_json::to_vec(&value).unwrap(),
                &s,
                RoutingLimits::default()
            )
            .is_err()
        );
    }
    let text = String::from_utf8(bytes).unwrap();
    let duplicate = text.replacen(
        &format!("\"schema_version\":{OWNED_ACTION_ROUTING_VERSION}"),
        &format!("\"schema_version\":{OWNED_ACTION_ROUTING_VERSION},\"schema_version\":{OWNED_ACTION_ROUTING_VERSION}"),
        1,
    );
    assert_ne!(duplicate, text);
    assert!(decode_action_routing(duplicate.as_bytes(), &s, RoutingLimits::default()).is_err());
    let mut version = base;
    version["schema_version"] = json!(OWNED_ACTION_ROUTING_VERSION + 1);
    assert!(matches!(
        decode_action_routing(
            &serde_json::to_vec(&version).unwrap(),
            &s,
            RoutingLimits::default()
        ),
        Err(RoutingError::Version(_))
    ));
}
#[test]
fn aggregate_and_wire_budgets_apply_to_construction_and_encoding() {
    let s = schema();
    let p = package(input(&s), &s);
    let limits = RoutingLimits {
        max_routes: 1,
        ..RoutingLimits::default()
    };
    assert!(OwnedActionRouting::new(input(&s), &s, limits).is_err());
    assert!(encode_action_routing(&p, limits).is_err());
    let limits = RoutingLimits {
        max_schema_work: 1,
        ..RoutingLimits::default()
    };
    assert!(OwnedActionRouting::new(input(&s), &s, limits).is_err());
    assert!(p.validate_limits(limits).is_err());
    let bytes = encode_action_routing(&p, RoutingLimits::default()).unwrap();
    let limits = RoutingLimits {
        max_wire_bytes: bytes.len() - 1,
        ..RoutingLimits::default()
    };
    assert!(decode_action_routing(&bytes, &s, limits).is_err());
    assert!(encode_action_routing(&p, limits).is_err());
    let mut i = input(&s);
    i.outputs.push(ActionOutputRoutes {
        source_selectors: Some(DeclaredSet::complete(vec![])),
        output: output("second"),
        routes: DeclaredSet::complete(vec![]),
    });
    assert!(
        OwnedActionRouting::new(
            i,
            &s,
            RoutingLimits {
                max_outputs: 1,
                ..RoutingLimits::default()
            }
        )
        .is_err()
    );
    let mut i = input(&s);
    i.outputs[0].routes.closure = SchemaClosure::Partial {
        gaps: vec![gap("first", "a"), gap("first", "b")],
    };
    assert!(
        OwnedActionRouting::new(
            i,
            &s,
            RoutingLimits {
                max_gaps: 1,
                ..RoutingLimits::default()
            }
        )
        .is_err()
    );
    assert!(
        RoutingLimits {
            max_routes: 0,
            ..RoutingLimits::default()
        }
        .validate()
        .is_err()
    );
}
#[test]
fn exact_schema_and_route_changes_invalidate_identity() {
    let s = schema();
    let p = package(input(&s), &s);
    p.verify_bindings(&s).unwrap();
    let mut raw = schema_input();
    raw.release = key("changed");
    let changed = OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap();
    assert!(p.verify_bindings(&changed).is_err());
    assert!(OwnedActionRouting::new(input(&s), &changed, RoutingLimits::default()).is_err());
    let mut i = input(&s);
    i.outputs[0].routes.members.remove(1);
    assert_ne!(p.identity(), package(i, &s).identity());
}
#[test]
fn inconsistent_custom_index_cannot_alias_source_stat_identity() {
    struct Inconsistent<'a>(&'a OwnedDefinitionSchemaPackage);
    impl DefinitionSchemaIndex for Inconsistent<'_> {
        fn identity(&self) -> &poe_optimizer_core::data::DataIdentity {
            self.0.identity()
        }
        fn namespace(&self) -> &GameVersionNamespace {
            self.0.namespace()
        }
        fn lookup_definition(&self, address: &DefinitionAddress) -> Option<&DefinitionDescriptor> {
            if address == &DefinitionAddress::Stat(id("equipment")) {
                self.0
                    .lookup_definition(&DefinitionAddress::Stat(id("actor")))
            } else {
                self.0.lookup_definition(address)
            }
        }
        fn lookup_slot(&self, address: &SlotAddress) -> Option<&SlotDescriptor> {
            self.0.lookup_slot(address)
        }
    }
    let s = schema();
    assert!(
        OwnedActionRouting::new(input(&s), &Inconsistent(&s), RoutingLimits::default())
            .unwrap_err()
            .to_string()
            .contains("inconsistent")
    );
}

fn selected_input() -> (OwnedDefinitionSchemaPackage, ActionRoutingInput) {
    let mut raw = schema_input();
    raw.definitions.push(DefinitionDescriptor::Capability(known(
        id("eligible"),
        CapabilitySchema {
            targets: vec![RuleEntityKind::EquipmentUse],
        },
    )));
    let s = OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap();
    let mut i = input(&s);
    i.outputs[0].source_selectors = Some(DeclaredSet::complete(vec![ActionSourceSelector {
        id: key("attack"),
        selection: ActionRouteSelection::All,
        sources: vec![
            NamedActionSource {
                id: key("weapon"),
                origin: ActionSourceOrigin::PlayerEquipment { slot: id("weapon") },
            },
            NamedActionSource {
                id: key("intrinsic"),
                origin: ActionSourceOrigin::ActionActor,
            },
            NamedActionSource {
                id: key("replacement"),
                origin: ActionSourceOrigin::CurrentAction,
            },
        ],
        policy: ActionSourcePolicy::EquipmentEligibility {
            source: key("weapon"),
            capability: id("eligible"),
            when_empty: ActionSourceOutcome::Use {
                source: key("intrinsic"),
            },
            when_ineligible: ActionSourceOutcome::Use {
                source: key("replacement"),
            },
        },
    }]));
    i.outputs[0].routes.members = vec![ActionStatRoute {
        id: key("selected"),
        selection: ActionRouteSelection::All,
        target: id("action"),
        source: ActionStatRouteSource::Selected {
            selector: key("attack"),
            stats: vec![
                ActionSourceStat {
                    source: key("weapon"),
                    stat: id("equipment"),
                },
                ActionSourceStat {
                    source: key("intrinsic"),
                    stat: id("actor"),
                },
                ActionSourceStat {
                    source: key("replacement"),
                    stat: id("action"),
                },
            ],
        },
    }];
    (s, i)
}
#[test]
fn selected_sources_roundtrip_canonicalize_and_preserve_v1_identity_bytes() {
    let (s, i) = selected_input();
    let p = package(i.clone(), &s);
    let mut reordered = i;
    reordered.outputs[0]
        .source_selectors
        .as_mut()
        .unwrap()
        .members[0]
        .sources
        .reverse();
    if let ActionStatRouteSource::Selected { stats, .. } =
        &mut reordered.outputs[0].routes.members[0].source
    {
        stats.reverse();
    }
    assert_eq!(p.identity(), package(reordered, &s).identity());
    let wire = encode_action_routing(&p, RoutingLimits::default()).unwrap();
    assert_eq!(
        decode_action_routing(&wire, &s, RoutingLimits::default())
            .unwrap()
            .identity(),
        p.identity()
    );
    let mut old = input(&s);
    old.schema_version = OWNED_ACTION_ROUTING_V1;
    old.outputs[0].source_selectors = None;
    old.outputs[0]
        .routes
        .members
        .sort_by(|a, b| a.id.cmp(&b.id));
    let bytes = serde_json::to_vec(&old).unwrap();
    assert!(
        !String::from_utf8(bytes.clone())
            .unwrap()
            .contains("source_selectors")
    );
    let expected = poe_optimizer_core::owned_content::digest_owned(
        "owned-action-routing-v1",
        &old,
        RoutingLimits::default().max_wire_bytes,
    )
    .unwrap();
    let checked = package(old.clone(), &s);
    assert_eq!(*checked.identity(), expected);
    assert_eq!(
        encode_action_routing(&checked, RoutingLimits::default()).unwrap(),
        bytes
    );
    old.outputs[0].source_selectors = Some(DeclaredSet::complete(vec![]));
    assert!(OwnedActionRouting::new(old, &s, RoutingLimits::default()).is_err());
    let mut null: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    null["outputs"][0]["source_selectors"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<ActionRoutingInput>(null).is_err());
}
#[test]
fn selected_source_bindings_are_exhaustive_typed_and_exactly_scoped() {
    let (s, original) = selected_input();
    for change in 0..10 {
        let mut i = original.clone();
        let output = &mut i.outputs[0];
        let selector = &mut output.source_selectors.as_mut().unwrap().members[0];
        let ActionStatRouteSource::Selected { stats, .. } = &mut output.routes.members[0].source
        else {
            panic!()
        };
        match change {
            0 => {
                stats.pop();
            }
            1 => stats[1] = stats[0].clone(),
            2 => stats[1].source = key("missing"),
            3 => stats[1].stat = id("equipment"),
            4 => stats[2].stat = id("action-other-unit"),
            5 => stats[2].stat = id("action-integer"),
            6 => selector.sources.push(NamedActionSource {
                id: key("unused"),
                origin: ActionSourceOrigin::ActionActor,
            }),
            7 => selector.sources[1].id = key("weapon"),
            8 => {
                selector.selection = ActionRouteSelection::Exact(Box::new(ActionRouteSelector {
                    part: id("part"),
                    mode: id("mode"),
                    stat_set: id("stats"),
                }))
            }
            9 => {
                selector.policy = ActionSourcePolicy::Fixed {
                    source: key("weapon"),
                }
            }
            _ => unreachable!(),
        }
        assert!(
            OwnedActionRouting::new(i, &s, RoutingLimits::default()).is_err(),
            "mutation {change}"
        );
    }
}
#[test]
fn source_selector_membership_eligibility_and_versions_never_default() {
    let (s, original) = selected_input();
    for change in 0..7 {
        let mut i = original.clone();
        match change {
            0 => i.outputs[0].source_selectors = None,
            1 => i.schema_version = OWNED_ACTION_ROUTING_V1,
            2 => {
                let selectors = i.outputs[0].source_selectors.as_mut().unwrap();
                selectors.members.push(selectors.members[0].clone());
            }
            3 => {
                i.outputs[0].source_selectors.as_mut().unwrap().closure =
                    SchemaClosure::Partial { gaps: vec![] }
            }
            4 | 5 => {
                if let ActionSourcePolicy::EquipmentEligibility { capability, .. } =
                    &mut i.outputs[0].source_selectors.as_mut().unwrap().members[0].policy
                {
                    *capability = if change == 4 {
                        id("missing")
                    } else {
                        DefId::new(
                            GameVersionNamespace::new("foreign", "v1").unwrap(),
                            key("eligible"),
                        )
                    };
                }
            }
            6 => {
                if let ActionSourcePolicy::EquipmentEligibility { when_empty, .. } =
                    &mut i.outputs[0].source_selectors.as_mut().unwrap().members[0].policy
                {
                    *when_empty = ActionSourceOutcome::Use {
                        source: key("weapon"),
                    };
                }
            }
            _ => unreachable!(),
        }
        assert!(
            OwnedActionRouting::new(i, &s, RoutingLimits::default()).is_err(),
            "mutation {change}"
        );
    }
    let mut i = original;
    i.outputs[0].source_selectors.as_mut().unwrap().closure = SchemaClosure::Partial {
        gaps: vec![gap("first", "unknown-selectors")],
    };
    assert!(OwnedActionRouting::new(i, &s, RoutingLimits::default()).is_ok());
}

#[test]
fn source_selector_resources_are_bounded_across_outputs_even_without_routes() {
    let (s, mut i) = selected_input();
    let p = package(i.clone(), &s);
    assert_eq!(p.resources().source_selectors, 1);
    assert_eq!(p.resources().named_sources, 3);
    assert_eq!(p.resources().source_bindings, 3);
    i.outputs[0].routes.members.clear();
    let mut second = i.outputs[0].clone();
    second.output = output("second");
    i.outputs.push(second);
    assert!(matches!(
        OwnedActionRouting::new(
            i,
            &s,
            RoutingLimits {
                max_routes: 1,
                ..RoutingLimits::default()
            }
        ),
        Err(RoutingError::Limit("source selectors"))
    ));
}
