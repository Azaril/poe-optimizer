//! Equipment receivers reuse the ordinary typed evaluator and concrete request graph.
#[allow(dead_code)]
#[path = "support/owned_plan_fixture.rs"]
mod support;
use poe_optimizer_core::{
    owned_build::*, owned_content::digest_owned, owned_definitions::*, owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::{
    owned_rules::{OwnedRulePackage, RuleStorageLimits},
    owned_schema::OwnedDefinitionSchemaPackage,
};
use poe_optimizer_engine::{
    owned_plan::*,
    owned_rules::{CompiledRulePackage, RuleLimits},
};
use std::sync::Arc;
use support::*;
fn stat_owner() -> SchemaSubject {
    subject(def::<StatDefinition>("local"))
}
fn receiver_program() -> RuleProgram {
    RuleProgram {
        id: key("assemble"),
        context: RuleEntityKind::EquipmentUse,
        reads: vec![contributions("incoming", RuleEntity::Current, "local")],
        nodes: vec![read_node("total", "incoming")],
        effects: vec![derive("final", RuleEntity::Current, "local", "total")],
    }
}
fn fixture() -> Fixture {
    let mut f = Fixture::new();
    let p = &mut f.owner_mut(&item_owner()).programs.members[0];
    p.reads.clear();
    p.nodes.retain(|n| n.id == key("base"));
    p.effects
        .retain(|e| matches!(e.effect, RuleEffectKind::Contribute { .. }));
    f.owners.push(DefinitionRules {
        owner: stat_owner(),
        programs: DeclaredSet::complete(vec![receiver_program()]),
    });
    f.receivers.members.push(StatReceiver {
        id: key("local-assembly"),
        stat: def("local"),
        program: key("assemble"),
        targets: vec![StatReceiverTarget::EquipmentTemplate {
            template: def("item"),
        }],
    });
    f
}
fn rules(f: &Fixture, version: &str) -> (OwnedDefinitionSchemaPackage, RulePackageInput) {
    let schema = OwnedDefinitionSchemaPackage::new(f.schema.clone(), Default::default()).unwrap();
    let input = RulePackageInput {
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: ns(),
        release: key("rules"),
        semantics_version: key("test-v1"),
        operations_version: key(version),
        definitions: schema.identity().clone(),
        tables: f.tables.clone(),
        owners: f.owners.clone(),
        receivers: f.receivers.clone(),
    };
    (schema, input)
}
fn report(f: &Fixture) -> OwnedEffectsReport {
    let p = f.compile().unwrap();
    p.evaluate(&mut p.new_scratch()).unwrap()
}
fn final_value(report: &OwnedEffectsReport, equipment: u64) -> Option<&EffectValue> {
    report
        .values
        .iter()
        .find(|row| {
            row.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::EquipmentUse(occurrence(equipment)),
                    stat: def("local"),
                }
        })
        .map(|row| &row.value)
}
fn known_value(report: &OwnedEffectsReport, equipment: u64, value: i64) {
    assert_eq!(
        final_value(report, equipment),
        Some(&EffectValue::Known {
            value: integer(value)
        })
    );
}

#[test]
fn shared_receiver_keeps_exact_item_modifier_and_equipment_occurrences() {
    let mut f = fixture();
    let before = report(&f);
    known_value(&before, 6, 18);
    known_value(&before, 7, 18);
    let receivers: Vec<_> = before
        .effects
        .iter()
        .filter_map(|effect| match effect.key.invocation.origin {
            RuleOrigin::EquipmentReceiver { equipment_use, .. } => Some(equipment_use),
            _ => None,
        })
        .collect();
    assert_eq!(receivers, [occurrence(6), occurrence(7)]);
    assert!(before.gaps.is_empty());
    f.build.items[0].modifiers[0].rolls[0].value = integer(31);
    let changed = report(&f);
    known_value(&changed, 6, 46);
    known_value(&changed, 7, 46);
    f.build.equipment[1].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(2)],
    };
    let inactive = report(&f);
    known_value(&inactive, 6, 46);
    assert!(final_value(&inactive, 7).is_none());
    f.build.active_weapon_loadout = occurrence(2);
    known_value(&report(&f), 7, 46);
}

#[test]
fn exact_template_applicability_does_not_create_or_guess_equipment() {
    let mut f = fixture();
    let template = f
        .schema
        .definitions
        .iter()
        .find_map(|d| {
            if let DefinitionDescriptor::ItemTemplate(e) = d {
                Some(e.clone())
            } else {
                None
            }
        })
        .unwrap();
    let mut other = template;
    other.id = def("other-template");
    let SchemaState::Known(s) = &mut other.schema else {
        unreachable!()
    };
    s.declarations.parameters.members.clear();
    f.schema
        .definitions
        .push(DefinitionDescriptor::ItemTemplate(other));
    let mut owner = f.owner_mut(&item_owner()).clone();
    owner.owner = subject(def::<ItemTemplateDefinition>("other-template"));
    f.owners.push(owner);
    f.receivers.members[0].targets = vec![StatReceiverTarget::EquipmentTemplate {
        template: def("other-template"),
    }];
    let absent = report(&f);
    assert!(final_value(&absent, 6).is_none());
    assert!(final_value(&absent, 7).is_none());
    assert!(!absent.effects.iter().any(|e| matches!(
        e.key.invocation.origin,
        RuleOrigin::EquipmentReceiver { .. }
    )));
    f.build.items[0].template = def("other-template");
    f.build.items[0].parameters.clear();
    known_value(&report(&f), 6, 18);
}

#[test]
fn complete_empty_channels_use_identity_but_partial_coverage_never_does() {
    let mut f = fixture();
    f.build.items[0].modifiers.clear();
    f.build.items[0].modifier_order.clear();
    f.owner_mut(&item_owner()).programs.members.clear();
    known_value(&report(&f), 6, 0);
    for owner in [stat_owner(), item_owner(), modifier_owner()] {
        let mut f = fixture();
        f.owner_mut(&owner).programs.closure = SchemaClosure::Partial {
            gaps: vec![SchemaGap {
                subject: owner.clone(),
                facet: SchemaFacet::GameRules,
                code: key("unknown-effects"),
            }],
        };
        assert!(matches!(
            final_value(&report(&f), 6),
            Some(EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                ..
            })
        ));
    }
    let mut f = fixture();
    f.receivers.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: stat_owner(),
            facet: SchemaFacet::GameRules,
            code: key("unknown-receivers"),
        }],
    };
    assert!(matches!(
        final_value(&report(&f), 6),
        Some(EffectValue::Unresolved {
            reason: PlanGapReason::IncompleteContributors,
            ..
        })
    ));
}

#[test]
fn existing_action_route_consumes_equipment_receiver_result_and_queries_do_not_discover_it() {
    let mut f = fixture();
    f.add_action_route();
    let first = report(&f);
    assert!(first.values.iter().any(|row| row.key
        == PlanValueKey::Stat {
            entity: ConcreteEntity::Action(Box::new(action())),
            stat: def("routed")
        }
        && row.value == EffectValue::Known { value: integer(18) }));
    f.queries.requests.clear();
    f.scenario.usage.clear();
    let unqueried = report(&f);
    known_value(&unqueried, 6, 18);
    known_value(&unqueried, 7, 18);
}

#[test]
fn duplicate_final_producers_and_receiver_cycles_still_reject() {
    let mut collision = fixture();
    collision.owner_mut(&item_owner()).programs.members[0]
        .effects
        .push(derive("conflict", RuleEntity::Current, "local", "base"));
    assert!(collision.compile().is_err());
    let mut cycle = fixture();
    cycle.owner_mut(&stat_owner()).programs.members[0].reads[0].source = RuleReadSource::Stat {
        entity: RuleEntity::Current,
        stat: def("local"),
    };
    assert!(cycle.compile().is_err());
}

#[test]
fn storage_and_compiler_reject_invalid_target_scope_context_and_old_versions() {
    for version in [
        OWNED_RULE_OPERATIONS_V6,
        OWNED_RULE_OPERATIONS_V7,
        OWNED_RULE_OPERATIONS_V8,
    ] {
        let (schema, input) = rules(&fixture(), version);
        assert!(OwnedRulePackage::new(input.clone(), &schema, Default::default()).is_err());
        assert!(
            CompiledRulePackage::compile(&input, &schema, Default::default())
                .unwrap_err()
                .message
                .contains("require owned-domain-operations-v9")
        );
    }
    for case in 0..8 {
        let mut f = fixture();
        match case {
            0 => f.receivers.members[0]
                .targets
                .push(StatReceiverTarget::Player),
            1 => {
                f.receivers.members[0].targets = vec![StatReceiverTarget::EquipmentTemplate {
                    template: def("missing"),
                }]
            }
            2 => {
                f.receivers.members[0].targets = vec![StatReceiverTarget::EquipmentTemplate {
                    template: DefId::parse(
                        GameVersionNamespace::new("foreign", "v1").unwrap(),
                        "item",
                    )
                    .unwrap(),
                }]
            }
            3 => {
                let target = f.receivers.members[0].targets[0].clone();
                f.receivers.members[0].targets.push(target);
            }
            4 => f.owner_mut(&stat_owner()).programs.members[0].context = RuleEntityKind::Actor,
            5 => {
                if let RuleEffectKind::Derive { entity, .. } =
                    &mut f.owner_mut(&stat_owner()).programs.members[0].effects[0].effect
                {
                    *entity = RuleEntity::Actor;
                }
            }
            6 => {
                let effect = f.owner_mut(&stat_owner()).programs.members[0].effects[0].clone();
                f.owner_mut(&stat_owner()).programs.members[0]
                    .effects
                    .push(RuleEffect {
                        id: key("second"),
                        ..effect
                    });
            }
            _ => {
                let template: ItemTemplateDefId = def("unmapped");
                f.schema
                    .definitions
                    .push(DefinitionDescriptor::ItemTemplate(DefinitionEntry {
                        id: template.clone(),
                        schema: SchemaState::Unmapped {
                            gaps: vec![SchemaGap {
                                subject: subject(template.clone()),
                                facet: SchemaFacet::InputSchema,
                                code: key("unknown"),
                            }],
                        },
                    }));
                f.receivers.members[0].targets =
                    vec![StatReceiverTarget::EquipmentTemplate { template }];
            }
        }
        let (schema, input) = rules(&f, OWNED_RULE_OPERATIONS_VERSION);
        assert!(
            OwnedRulePackage::new(input.clone(), &schema, Default::default()).is_err(),
            "storage case {case}"
        );
        assert!(
            CompiledRulePackage::compile(&input, &schema, Default::default()).is_err(),
            "compile case {case}"
        );
    }
}

#[test]
fn receiver_does_not_acquire_template_level_quality_or_modifier_read_authority() {
    for source in [
        RuleReadSource::ItemLevel,
        RuleReadSource::HasItemQuality {
            quality: def("ordinary"),
        },
        RuleReadSource::ItemQualityAmount {
            quality: def("ordinary"),
        },
        RuleReadSource::Parameter {
            slot: parameter(SlotOwnerDefId::Modifier(def("modifier")), "roll"),
        },
    ] {
        let mut f = fixture();
        f.owner_mut(&stat_owner()).programs.members[0]
            .reads
            .push(RuleRead {
                id: key("forbidden"),
                value_type: ComputedValueType::Integer,
                source,
            });
        let (schema, input) = rules(&f, OWNED_RULE_OPERATIONS_VERSION);
        assert!(CompiledRulePackage::compile(&input, &schema, Default::default()).is_err());
    }
}

#[test]
fn old_actor_wire_and_v6_v7_v8_canonical_compiled_identities_are_unchanged() {
    let target = ActorReceiverTarget::Player;
    assert_eq!(
        serde_json::to_string(&target).unwrap(),
        r#"{"kind":"player"}"#
    );
    let _: StatReceiverTarget = target;
    for version in [
        OWNED_RULE_OPERATIONS_V6,
        OWNED_RULE_OPERATIONS_V7,
        OWNED_RULE_OPERATIONS_V8,
    ] {
        let (schema, input) = rules(&Fixture::new(), version);
        let compiled = CompiledRulePackage::compile(&input, &schema, Default::default()).unwrap();
        let canonical = compiled.input().clone();
        let bytes = serde_json::to_vec(&canonical).unwrap();
        assert_eq!(
            compiled.identity(),
            digest_owned(
                "owned-rule-programs-v2",
                &canonical,
                RuleLimits::default().max_wire_bytes
            )
            .unwrap()
        );
        let again = CompiledRulePackage::compile(
            &serde_json::from_slice(&bytes).unwrap(),
            &schema,
            Default::default(),
        )
        .unwrap();
        assert_eq!(again.identity(), compiled.identity());
        assert_eq!(serde_json::to_vec(again.input()).unwrap(), bytes);
    }
}

#[test]
fn receiver_expansion_is_bounded_and_worker_scratch_is_independent() {
    let f = fixture();
    assert!(
        f.compile_with(PlanLimits {
            max_effects: 1,
            ..Default::default()
        })
        .is_err()
    );
    let (schema, input) = rules(&f, OWNED_RULE_OPERATIONS_VERSION);
    assert!(
        OwnedRulePackage::new(
            input.clone(),
            &schema,
            RuleStorageLimits {
                max_receiver_targets: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
    let plan = Arc::new(f.compile().unwrap());
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let plan = Arc::clone(&plan);
            scope.spawn(move || {
                let mut scratch = plan.new_scratch();
                for _ in 0..3 {
                    known_value(&plan.evaluate(&mut scratch).unwrap(), 6, 18);
                }
            });
        }
    });
}

#[test]
fn socketed_equipment_receiver_inherits_container_activity() {
    let mut f = fixture();
    let socket: SocketSlotDefId = def("item-socket");
    f.schema
        .definitions
        .push(DefinitionDescriptor::SocketSlot(DefinitionEntry {
            id: socket.clone(),
            schema: SchemaState::Known(SocketSlotSchema {
                owner: SlotOwnerDefId::ItemTemplate(def("item")),
                kind: SocketKind::Item,
                scope: ScopePolicy::Either,
            }),
        }));
    for row in &mut f.schema.definitions {
        if let DefinitionDescriptor::ItemTemplate(entry) = row
            && let SchemaState::Known(template) = &mut entry.schema
        {
            template.declarations.sockets.members.push(socket.clone());
            template.socket_destinations.members.push(socket.clone());
        }
    }
    f.build.equipment[0].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(2)],
    };
    f.build.equipment[1].destination = EquipmentDestination::ItemSocket {
        container: occurrence(6),
        slot: socket,
    };
    let inactive = report(&f);
    assert!(final_value(&inactive, 6).is_none());
    assert!(final_value(&inactive, 7).is_none());
    f.build.active_weapon_loadout = occurrence(2);
    let active = report(&f);
    known_value(&active, 6, 18);
    known_value(&active, 7, 18);
}
