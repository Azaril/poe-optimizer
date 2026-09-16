//! Native identity predicates bind the request's character, never injected RuleFacts.
#[path = "support/owned_plan_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits};
use poe_optimizer_engine::{
    owned_plan::*,
    owned_rules::{CompiledRulePackage, RuleLimits},
};
use std::{fs, path::PathBuf, sync::Arc};

fn known<I, D>(id: I, schema: D) -> DefinitionEntry<I, D> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn bool_read(id: &str, source: RuleReadSource) -> RuleRead {
    RuleRead {
        id: key(id),
        value_type: ComputedValueType::Boolean,
        source,
    }
}
fn predicates(context: RuleEntityKind) -> RuleProgram {
    RuleProgram {
        id: key("character-predicates"),
        context,
        reads: vec![
            bool_read(
                "class",
                RuleReadSource::CharacterClassIs {
                    class: def("class"),
                },
            ),
            bool_read(
                "ascendancy",
                RuleReadSource::CharacterAscendancyIs {
                    ascendancy: def("asc-a"),
                },
            ),
        ],
        nodes: vec![
            read_node("class", "class"),
            read_node("ascendancy", "ascendancy"),
        ],
        effects: ["class", "ascendancy"]
            .into_iter()
            .map(|name| {
                effect(
                    name,
                    RuleEffectKind::Requirement {
                        satisfied: key(name),
                        code: key("identity-observation"),
                    },
                )
            })
            .collect(),
    }
}
fn fixture() -> Fixture {
    let mut f = Fixture::new();
    let class = f
        .schema
        .definitions
        .iter_mut()
        .find_map(|d| match d {
            DefinitionDescriptor::Class(entry) => match &mut entry.schema {
                SchemaState::Known(s) => Some(s),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    class.ascendancies = DeclaredSet::complete(vec![def("asc-a"), def("asc-b")]);
    let other = class.clone();
    let declarations = class.declarations.clone();
    f.schema.definitions.push(DefinitionDescriptor::Class(known(
        def("other-class"),
        other,
    )));
    for name in ["asc-a", "asc-b"] {
        f.schema
            .definitions
            .push(DefinitionDescriptor::Ascendancy(known(
                def(name),
                AscendancySchema {
                    classes: DeclaredSet::complete(vec![def("class"), def("other-class")]),
                    implicit_passives: DeclaredSet::complete(vec![]),
                    declarations: declarations.clone(),
                },
            )));
        f.owners.push(DefinitionRules {
            owner: subject(def::<AscendancyDefinition>(name)),
            programs: DeclaredSet::complete(vec![]),
        });
    }
    let mut second_owner = f.owner_mut(&class_owner()).clone();
    second_owner.owner = subject(def::<ClassDefinition>("other-class"));
    f.owners.push(second_owner);
    f.schema.definitions.push(DefinitionDescriptor::Stat(known(
        def("identity-selection"),
        StatSchema {
            value: ComputedValueType::Integer,
            targets: vec![RuleEntityKind::Actor],
        },
    )));
    for owner in [
        class_owner(),
        subject(def::<ClassDefinition>("other-class")),
    ] {
        let mut p = predicates(RuleEntityKind::Actor);
        p.nodes.extend([
            literal("class-value", 11),
            literal("asc-value", 22),
            literal("other", 33),
            node(
                "asc-selection",
                RuleExpression::Select {
                    condition: key("ascendancy"),
                    when_true: key("asc-value"),
                    when_false: key("other"),
                },
            ),
            node(
                "class-selection",
                RuleExpression::Select {
                    condition: key("class"),
                    when_true: key("class-value"),
                    when_false: key("asc-selection"),
                },
            ),
        ]);
        p.effects.push(derive(
            "selected",
            RuleEntity::Player,
            "identity-selection",
            "class-selection",
        ));
        f.owner_mut(&owner).programs.members.push(p);
    }
    f.owner_mut(&item_owner())
        .programs
        .members
        .push(predicates(RuleEntityKind::EquipmentUse));
    f.owner_mut(&modifier_owner())
        .programs
        .members
        .push(predicates(RuleEntityKind::EquipmentUse));
    for context in [RuleEntityKind::Enemy, RuleEntityKind::Environment] {
        let mut p = predicates(context);
        p.id = key(if context == RuleEntityKind::Enemy {
            "character-predicates-enemy"
        } else {
            "character-predicates-environment"
        });
        f.owner_mut(&subject(def::<EncounterDefinition>("encounter")))
            .programs
            .members
            .push(p);
    }
    f.add_action_route();
    f.owner_mut(&SchemaSubject::Slot(SlotAddress::ActionOutput(output())))
        .programs
        .members
        .push(predicates(RuleEntityKind::Action));
    f.add_generated_actors();
    f.owner_mut(&SchemaSubject::Slot(SlotAddress::Actor(child_slot())))
        .programs
        .members
        .push(predicates(RuleEntityKind::Actor));
    f.add_level_program();
    f
}
fn schema(f: &Fixture) -> OwnedDefinitionSchemaPackage {
    OwnedDefinitionSchemaPackage::new(f.schema.clone(), OwnedSchemaLimits::default()).unwrap()
}
fn rules(f: &Fixture, schema: &OwnedDefinitionSchemaPackage) -> RulePackageInput {
    RulePackageInput {
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: ns(),
        release: key("identity-test"),
        semantics_version: key("v1"),
        operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
        definitions: schema.identity().clone(),
        tables: f.tables.clone(),
        owners: f.owners.clone(),
        receivers: f.receivers.clone(),
    }
}
fn selection(report: &OwnedEffectsReport) -> &EffectValue {
    &report
        .values
        .iter()
        .find(|r| {
            r.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: def("identity-selection"),
                }
        })
        .unwrap()
        .value
}
fn assert_predicates(report: &OwnedEffectsReport, class: bool, ascendancy: bool) {
    let effects: Vec<_> = report
        .effects
        .iter()
        .filter(|e| {
            e.key
                .invocation
                .program
                .as_str()
                .starts_with("character-predicates")
                && e.key.effect != key("selected")
        })
        .collect();
    assert_eq!(
        effects.len(),
        24,
        "character, two equipment, four modifiers, action, two owned actors, enemy and environment"
    );
    for e in effects {
        let expected = if e.key.effect == key("class") {
            class
        } else {
            assert_eq!(e.key.effect, key("ascendancy"));
            ascendancy
        };
        assert_eq!(
            e.value,
            EffectValue::Known {
                value: ParameterValue::Boolean(expected)
            },
            "{e:?}"
        );
    }
}

#[test]
fn predicates_follow_request_class_ascendancy_and_none_in_each_active_context() {
    let mut f = fixture();
    let mut identities = std::collections::BTreeSet::new();
    for (class, asc, class_matches, asc_matches, expected) in [
        ("class", Some("asc-a"), true, true, 11),
        ("class", Some("asc-b"), true, false, 11),
        ("class", None, true, false, 11),
        ("other-class", Some("asc-a"), false, true, 22),
        ("other-class", None, false, false, 33),
    ] {
        f.build.character.class = def(class);
        f.build.character.ascendancy = asc.map(def);
        let plan = f.compile().unwrap();
        assert!(
            identities.insert(plan.identity()),
            "changed authored identity must change plan identity"
        );
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert_predicates(&report, class_matches, asc_matches);
        assert_eq!(
            selection(&report),
            &EffectValue::Known {
                value: integer(expected)
            }
        );
        assert!(
            report
                .effects
                .iter()
                .any(|e| e.key.invocation.entity == ConcreteEntity::Actor(child_actor(30)))
        );
        assert!(report.gaps.is_empty(), "{report:?}");
    }
}

#[test]
fn typed_references_reject_missing_unmapped_foreign_and_wrong_declared_types() {
    let f = fixture();
    let checked = schema(&f);
    for source in [
        RuleReadSource::CharacterClassIs {
            class: def("missing-class"),
        },
        RuleReadSource::CharacterAscendancyIs {
            ascendancy: def("missing-ascendancy"),
        },
        RuleReadSource::CharacterClassIs {
            class: ClassDefId::parse(GameVersionNamespace::new("foreign", "v1").unwrap(), "class")
                .unwrap(),
        },
        RuleReadSource::CharacterAscendancyIs {
            ascendancy: AscendancyDefId::parse(
                GameVersionNamespace::new("foreign", "v1").unwrap(),
                "asc-a",
            )
            .unwrap(),
        },
    ] {
        let mut input = rules(&f, &checked);
        let p = input
            .owners
            .iter_mut()
            .flat_map(|o| &mut o.programs.members)
            .find(|p| p.id == key("character-predicates"))
            .unwrap();
        p.reads[0].source = source;
        assert!(
            CompiledRulePackage::compile(&input, &checked, RuleLimits::default())
                .unwrap_err()
                .to_string()
                .contains("missing, unmapped, foreign")
        );
    }
    for source in [
        RuleReadSource::CharacterClassIs {
            class: def("class"),
        },
        RuleReadSource::CharacterAscendancyIs {
            ascendancy: def("asc-a"),
        },
    ] {
        let mut input = rules(&f, &checked);
        let p = input
            .owners
            .iter_mut()
            .flat_map(|o| &mut o.programs.members)
            .find(|p| p.id == key("character-predicates"))
            .unwrap();
        p.reads[0].source = source;
        p.reads[0].value_type = ComputedValueType::Integer;
        assert!(
            CompiledRulePackage::compile(&input, &checked, RuleLimits::default())
                .unwrap_err()
                .to_string()
                .contains("declared read type")
        );
    }
    for is_class in [true, false] {
        let mut raw = f.schema.clone();
        let gap = |subject| SchemaGap {
            subject,
            facet: SchemaFacet::InputSchema,
            code: key("unmapped-fixture"),
        };
        if is_class {
            raw.definitions
                .push(DefinitionDescriptor::Class(DefinitionEntry {
                    id: def("unmapped"),
                    schema: SchemaState::Unmapped {
                        gaps: vec![gap(subject(def::<ClassDefinition>("unmapped")))],
                    },
                }));
        } else {
            raw.definitions
                .push(DefinitionDescriptor::Ascendancy(DefinitionEntry {
                    id: def("unmapped"),
                    schema: SchemaState::Unmapped {
                        gaps: vec![gap(subject(def::<AscendancyDefinition>("unmapped")))],
                    },
                }));
        }
        let checked = OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap();
        let mut input = rules(&f, &checked);
        let p = input
            .owners
            .iter_mut()
            .flat_map(|o| &mut o.programs.members)
            .find(|p| p.id == key("character-predicates"))
            .unwrap();
        p.reads[0].source = if is_class {
            RuleReadSource::CharacterClassIs {
                class: def("unmapped"),
            }
        } else {
            RuleReadSource::CharacterAscendancyIs {
                ascendancy: def("unmapped"),
            }
        };
        assert!(
            CompiledRulePackage::compile(&input, &checked, RuleLimits::default())
                .unwrap_err()
                .to_string()
                .contains("missing, unmapped, foreign")
        );
    }
    for malformed in [
        r#"{"kind":"character_class_is","value":{}}"#,
        r#"{"kind":"character_ascendancy_is","value":{"class":{}}}"#,
        r#"{"kind":"character_class_is","value":{"class":null}}"#,
    ] {
        assert!(serde_json::from_str::<RuleReadSource>(malformed).is_err());
    }
}

#[test]
fn v7_is_explicit_and_v6_cannot_admit_either_new_predicate() {
    let f = fixture();
    let checked = schema(&f);
    let mut input = rules(&f, &checked);
    input.operations_version = key(OWNED_RULE_OPERATIONS_V7);
    assert!(CompiledRulePackage::compile(&input, &checked, RuleLimits::default()).is_ok());
    for source in [
        RuleReadSource::CharacterClassIs {
            class: def("class"),
        },
        RuleReadSource::CharacterAscendancyIs {
            ascendancy: def("asc-a"),
        },
    ] {
        let mut legacy = input.clone();
        legacy.operations_version = key(OWNED_RULE_OPERATIONS_V6);
        for p in legacy
            .owners
            .iter_mut()
            .flat_map(|o| &mut o.programs.members)
        {
            for r in &mut p.reads {
                if matches!(
                    r.source,
                    RuleReadSource::CharacterClassIs { .. }
                        | RuleReadSource::CharacterAscendancyIs { .. }
                ) {
                    r.source = source.clone();
                }
            }
        }
        assert!(
            CompiledRulePackage::compile(&legacy, &checked, RuleLimits::default())
                .unwrap_err()
                .to_string()
                .contains("predicates require owned-domain-operations-v7")
        );
    }
    for unsupported in [
        "owned-domain-operations-v5",
        "owned-domain-operations-v10",
        "different-operations",
    ] {
        let mut bad = input.clone();
        bad.operations_version = key(unsupported);
        assert!(
            CompiledRulePackage::compile(&bad, &checked, RuleLimits::default())
                .unwrap_err()
                .to_string()
                .contains("unsupported operation version")
        );
    }
}

#[test]
fn persisted_v6_compiled_identity_and_input_are_unchanged() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/owned/poe2/3887ae68/current");
    let schema = OwnedDefinitionSchemaPackage::new(
        serde_json::from_slice(&fs::read(path.join("schema.json")).unwrap()).unwrap(),
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let input: RulePackageInput =
        serde_json::from_slice(&fs::read(path.join("rules.json")).unwrap()).unwrap();
    assert_eq!(input.operations_version.as_str(), OWNED_RULE_OPERATIONS_V6);
    let before = serde_json::to_value(&input).unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(path.join("manifest.json")).unwrap()).unwrap();
    let compiled = CompiledRulePackage::compile(&input, &schema, RuleLimits::default()).unwrap();
    assert_eq!(
        serde_json::to_value(compiled.identity()).unwrap(),
        manifest["compiled_rules"]
    );
    // Compilation has always canonicalized declaration order. The persisted
    // compiled digest above proves that canonical form remains byte-identical.
    assert_eq!(
        compiled.input().operations_version,
        input.operations_version
    );
    assert!(
        serde_json::to_value(&input).unwrap() == before,
        "caller input changed"
    );
}

#[test]
fn plans_are_immutable_parallel_and_reused_scratch_cannot_retain_prior_character() {
    let mut f = fixture();
    f.build.character.ascendancy = Some(def("asc-a"));
    let matched = Arc::new(f.compile().unwrap());
    f.build.character.class = def("other-class");
    f.build.character.ascendancy = None;
    let different = Arc::new(f.compile().unwrap());
    assert_ne!(matched.bindings().request, different.bindings().request);
    assert_eq!(matched.bindings().rules, different.bindings().rules);
    assert_eq!(
        matched.bindings().definitions,
        different.bindings().definitions
    );
    let mut scratch = matched.new_scratch();
    for p in [&matched, &different, &matched, &different] {
        let report = p.evaluate(&mut scratch).unwrap();
        let is_match =
            p.request().build().input().character.class == def::<ClassDefinition>("class");
        assert_predicates(&report, is_match, is_match);
        assert_eq!(
            selection(&report),
            &EffectValue::Known {
                value: integer(if is_match { 11 } else { 33 })
            }
        );
    }
    let handles: Vec<_> = (0..8)
        .map(|i| {
            let p = if i % 2 == 0 {
                matched.clone()
            } else {
                different.clone()
            };
            std::thread::spawn(move || {
                let mut scratch = p.new_scratch();
                for _ in 0..16 {
                    let report = p.evaluate(&mut scratch).unwrap();
                    assert_predicates(&report, i % 2 == 0, i % 2 == 0);
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn predicates_do_not_activate_disabled_owned_actors_or_close_missing_rules() {
    let mut f = fixture();
    f.build
        .skills
        .iter_mut()
        .find(|s| s.id == occurrence(30))
        .unwrap()
        .enabled = false;
    let plan = f.compile().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(
        !report
            .effects
            .iter()
            .any(|e| e.key.invocation.entity == ConcreteEntity::Actor(child_actor(30)))
    );
    let gap = SchemaGap {
        subject: modifier_owner(),
        facet: SchemaFacet::GameRules,
        code: key("not-implemented"),
    };
    f.owner_mut(&modifier_owner()).programs.closure = SchemaClosure::Partial { gaps: vec![gap] };
    let p = f.compile().unwrap();
    let report = p.evaluate(&mut p.new_scratch()).unwrap();
    assert!(!report.gaps.is_empty());
    assert!(report.values.iter().any(|r| r.key
        == PlanValueKey::Stat {
            entity: ConcreteEntity::Actor(ActorKey::Player),
            stat: def("actor-total")
        }
        && matches!(r.value, EffectValue::Unresolved { .. })));
}
