//! Independent, directly authored schema-package laws. No source checkout/runtime.
use poe_optimizer_core::{owned_build::DeclaredSlot, owned_definitions::*, owned_schema::*};
use poe_optimizer_data::owned_schema::*;
use serde_json::{Value, json};

fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("authored-game", "v1").unwrap()
}
fn id<K: DefinitionDomain>(key: &str) -> DefId<K> {
    DefId::parse(ns(), key).unwrap()
}
fn key(key: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(key).unwrap()
}
fn item_owner() -> SlotOwnerDefId {
    SlotOwnerDefId::ItemTemplate(id("item"))
}
fn declared<K: DefinitionDomain>(owner: SlotOwnerDefId, name: &str) -> DeclaredSlot<DefId<K>> {
    DeclaredSlot {
        declaration: owner,
        slot: id(name),
    }
}
fn range(minimum: i64, maximum: i64) -> IntegerRange {
    IntegerRange {
        minimum: BoundedInteger::new(minimum).unwrap(),
        maximum: BoundedInteger::new(maximum).unwrap(),
    }
}
fn quantity(minimum: f64, maximum: f64, unit: &str) -> QuantityRange {
    QuantityRange {
        minimum: FiniteQuantity::new(minimum, id(unit)).unwrap(),
        maximum: FiniteQuantity::new(maximum, id(unit)).unwrap(),
    }
}
fn declarations() -> DeclaredSlots {
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
fn gap(subject: SchemaSubject) -> SchemaGap {
    SchemaGap {
        subject,
        facet: SchemaFacet::StaticLinks,
        code: key("unconverted-link"),
    }
}
fn known<I, D>(id: I, schema: D) -> DefinitionEntry<I, D> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn quality() -> QualityUseSchema {
    QualityUseSchema {
        presence: QualityPresence::Optional,
        allowed_kinds: DeclaredSet::complete(vec![id("quality")]),
    }
}

fn input() -> SchemaPackageInput {
    let item_parameter = declared(item_owner(), "item-state");
    let gem_parameter = declared(SlotOwnerDefId::Gem(id("gem")), "gem-variant");
    let modifier_parameter = declared(SlotOwnerDefId::Modifier(id("modifier")), "roll");
    let reward_parameter = declared(SlotOwnerDefId::Reward(id("reward")), "reward-count");
    let usage_parameter = declared(SlotOwnerDefId::UsagePolicy(id("usage")), "duration");
    let choice = declared(item_owner(), "action-choice");
    let grant = declared(item_owner(), "actor-grant");
    let actor = declared(item_owner(), "owned-actor");
    let skill_grant = declared(item_owner(), "skill-grant");
    let output = declared(item_owner(), "output");
    let mut item_declarations = declarations();
    item_declarations
        .parameters
        .members
        .push(item_parameter.clone());
    item_declarations.choices.members.push(choice.clone());
    item_declarations.grants.members.push(grant.clone());
    item_declarations.actors.members.push(actor.clone());
    item_declarations
        .skill_grants
        .members
        .push(skill_grant.clone());
    item_declarations.outputs.members.push(output.clone());
    item_declarations.sockets.members.push(id("item-socket"));
    let mut gem_declarations = declarations();
    gem_declarations
        .parameters
        .members
        .push(gem_parameter.clone());
    let mut modifier_declarations = declarations();
    modifier_declarations
        .parameters
        .members
        .push(modifier_parameter.clone());
    let mut reward_declarations = declarations();
    reward_declarations
        .parameters
        .members
        .push(reward_parameter.clone());
    let mut usage_declarations = declarations();
    usage_declarations
        .parameters
        .members
        .push(usage_parameter.clone());
    let mut node_declarations = declarations();
    node_declarations.sockets.members.push(id("passive-socket"));
    let definitions = vec![
        DefinitionDescriptor::Class(known(
            id("class"),
            ClassSchema {
                implicit_passives: DeclaredSet::complete(vec![]),
                level: range(1, 100),
                ascendancies: DeclaredSet::complete(vec![id("ascendancy")]),
                declarations: declarations(),
            },
        )),
        DefinitionDescriptor::Ascendancy(known(
            id("ascendancy"),
            AscendancySchema {
                implicit_passives: DeclaredSet::complete(vec![]),
                classes: DeclaredSet::complete(vec![id("class")]),
                declarations: declarations(),
            },
        )),
        DefinitionDescriptor::Reward(known(
            id("reward"),
            RewardSchema {
                declarations: reward_declarations,
            },
        )),
        DefinitionDescriptor::ItemTemplate(known(
            id("item"),
            ItemTemplateSchema {
                item_level: range(0, 100),
                equipment_slots: DeclaredSet::complete(vec![id("equipment-slot")]),
                socket_destinations: DeclaredSet::complete(vec![
                    id("passive-socket"),
                    id("item-socket"),
                ]),
                modifiers: DeclaredSet::complete(vec![id("modifier")]),
                quality: quality(),
                declarations: item_declarations,
            },
        )),
        DefinitionDescriptor::Modifier(known(
            id("modifier"),
            ModifierSchema {
                declarations: modifier_declarations,
            },
        )),
        DefinitionDescriptor::Gem(known(
            id("gem"),
            GemSchema {
                level: range(1, 30),
                roles: vec![
                    AuthoredGemRole::SupportAssignment,
                    AuthoredGemRole::SkillUse,
                ],
                skills: DeclaredSet::complete(vec![id("skill")]),
                quality: quality(),
                declarations: gem_declarations,
            },
        )),
        DefinitionDescriptor::Skill(known(
            id("skill"),
            SkillSchema {
                directly_selectable: true,
                declarations: declarations(),
            },
        )),
        DefinitionDescriptor::PassiveNode(known(
            id("node"),
            PassiveNodeSchema {
                pools: DeclaredSet::complete(vec![id("pool")]),
                adjacent: DeclaredSet::complete(vec![id("node")]),
                declarations: node_declarations,
            },
        )),
        DefinitionDescriptor::PointPool(known(
            id("pool"),
            PointPoolSchema {
                scope: PointPoolScope::PerLoadout,
            },
        )),
        DefinitionDescriptor::EquipmentSlot(known(
            id("equipment-slot"),
            EquipmentSlotSchema {
                scope: ScopePolicy::Either,
            },
        )),
        DefinitionDescriptor::SocketSlot(known(
            id("item-socket"),
            SocketSlotSchema {
                owner: item_owner(),
                kind: SocketKind::Item,
                scope: ScopePolicy::Either,
            },
        )),
        DefinitionDescriptor::SocketSlot(known(
            id("passive-socket"),
            SocketSlotSchema {
                owner: SlotOwnerDefId::PassiveNode(id("node")),
                kind: SocketKind::Passive,
                scope: ScopePolicy::Shared,
            },
        )),
        DefinitionDescriptor::Encounter(known(
            id("encounter"),
            EncounterSchema {
                enemy_level: range(1, 100),
                external_inputs: DeclaredSet::complete(vec![id("external")]),
            },
        )),
        DefinitionDescriptor::Metric(known(
            id("metric"),
            MetricSchema {
                targets: vec![MetricTargetKind::Action, MetricTargetKind::Actor],
                unit: id("ratio"),
                actor_roles: vec![MetricActorRole::Owned, MetricActorRole::Player],
                provider_roles: vec![
                    ProviderRole::SupportAssignment,
                    ProviderRole::ItemModifier,
                    ProviderRole::EquipmentUse,
                ],
            },
        )),
        DefinitionDescriptor::Stat(known(
            id("stat"),
            StatSchema {
                value: ComputedValueType::Quantity { unit: id("ratio") },
                targets: vec![RuleEntityKind::Actor, RuleEntityKind::EquipmentUse],
            },
        )),
        DefinitionDescriptor::Capability(known(
            id("capability"),
            CapabilitySchema {
                targets: vec![RuleEntityKind::Action, RuleEntityKind::Actor],
            },
        )),
        DefinitionDescriptor::Option(known(id("option"), OptionSchema {})),
        DefinitionDescriptor::ActionPart(known(id("part"), ActionPartSchema {})),
        DefinitionDescriptor::ActionMode(known(id("mode"), ActionModeSchema {})),
        DefinitionDescriptor::ActionStatSet(known(id("stat-set"), ActionStatSetSchema {})),
        DefinitionDescriptor::UsagePolicy(known(
            id("usage"),
            UsagePolicySchema {
                targets: vec![
                    UsageTargetKind::Skill,
                    UsageTargetKind::Action,
                    UsageTargetKind::Actor,
                ],
                declarations: usage_declarations,
            },
        )),
        DefinitionDescriptor::SkillLinkRole(known(
            id("link-role"),
            SkillLinkRoleSchema {
                containers: DeclaredSet::complete(vec![id("skill")]),
                payloads: DeclaredSet::complete(vec![id("skill")]),
            },
        )),
        DefinitionDescriptor::Unit(known(
            id("ratio"),
            UnitSchema {
                dimension: UnitDimension::DimensionlessFactor,
            },
        )),
        DefinitionDescriptor::Unit(known(
            id("other-ratio"),
            UnitSchema {
                dimension: UnitDimension::DimensionlessFactor,
            },
        )),
        DefinitionDescriptor::Unit(known(
            id("time"),
            UnitSchema {
                dimension: UnitDimension::Time,
            },
        )),
        DefinitionDescriptor::Quality(known(
            id("quality"),
            QualitySchema {
                amount: quantity(0.0, 20.0, "ratio"),
            },
        )),
        DefinitionDescriptor::ExternalInput(known(
            id("external"),
            ExternalInputSchema {
                value: ValueSchema::Boolean,
                targets: vec![
                    AssumptionTargetKind::Enemy,
                    AssumptionTargetKind::Environment,
                ],
            },
        )),
    ];
    let slots = vec![
        SlotDescriptor::Parameter(known(
            item_parameter,
            ParameterSlotSchema {
                value: ValueSchema::Boolean,
                presence: SlotPresence::OptionalOnce,
                sites: vec![ParameterSite::ItemParameter],
            },
        )),
        SlotDescriptor::Parameter(known(
            gem_parameter,
            ParameterSlotSchema {
                value: ValueSchema::Option {
                    allowed: DeclaredSet::complete(vec![id("option")]),
                },
                presence: SlotPresence::OptionalOnce,
                sites: vec![ParameterSite::GemParameter],
            },
        )),
        SlotDescriptor::Parameter(known(
            modifier_parameter,
            ParameterSlotSchema {
                value: ValueSchema::Quantity(quantity(-3.5, 9.5, "ratio")),
                presence: SlotPresence::RequiredOnce,
                sites: vec![ParameterSite::ModifierRoll],
            },
        )),
        SlotDescriptor::Parameter(known(
            reward_parameter,
            ParameterSlotSchema {
                value: ValueSchema::Integer(range(0, 9)),
                presence: SlotPresence::RequiredOnce,
                sites: vec![ParameterSite::RewardParameter],
            },
        )),
        SlotDescriptor::Parameter(known(
            usage_parameter,
            ParameterSlotSchema {
                value: ValueSchema::Quantity(quantity(0.0, 100.0, "time")),
                presence: SlotPresence::RequiredOnce,
                sites: vec![ParameterSite::UsagePolicyParameter],
            },
        )),
        SlotDescriptor::Choice(known(
            choice.clone(),
            ChoiceSlotSchema {
                value: ValueSchema::Option {
                    allowed: DeclaredSet::complete(vec![id("option")]),
                },
                presence: SlotPresence::OptionalOnce,
                owners: vec![
                    ChoiceOwnerScope::Action,
                    ChoiceOwnerScope::Provider(ProviderRole::EquipmentUse),
                ],
            },
        )),
        SlotDescriptor::Grant(known(
            grant,
            GrantSlotSchema {
                provider_roles: vec![ProviderRole::EquipmentUse],
                target: GrantTarget::Actor(actor.clone()),
            },
        )),
        SlotDescriptor::Actor(known(
            actor.clone(),
            ActorSlotSchema {
                skills: DeclaredSet::complete(vec![id("skill")]),
                outputs: DeclaredSet::complete(vec![output.clone()]),
            },
        )),
        SlotDescriptor::SkillGrant(known(
            skill_grant,
            SkillGrantSlotSchema {
                skill: id("skill"),
                outputs: DeclaredSet::complete(vec![output.clone()]),
            },
        )),
        SlotDescriptor::ActionOutput(known(
            output,
            ActionOutputSchema {
                actor_role: DeclaredActorRole::OwnedSlot(actor),
                parts: DeclaredSet::complete(vec![id("part")]),
                modes: DeclaredSet::complete(vec![id("mode")]),
                stat_sets: DeclaredSet::complete(vec![id("stat-set")]),
                choices: DeclaredSet::complete(vec![choice]),
            },
        )),
    ];
    SchemaPackageInput {
        schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
        namespace: ns(),
        release: key("release-a"),
        semantics_version: key("schema-semantics-1"),
        definitions,
        slots,
    }
}

fn package(input: SchemaPackageInput) -> OwnedDefinitionSchemaPackage {
    OwnedDefinitionSchemaPackage::new(input, OwnedSchemaLimits::default()).unwrap()
}
fn item(input: &mut SchemaPackageInput) -> &mut ItemTemplateSchema {
    input
        .definitions
        .iter_mut()
        .find_map(|d| match d {
            DefinitionDescriptor::ItemTemplate(DefinitionEntry {
                schema: SchemaState::Known(s),
                ..
            }) => Some(s),
            _ => None,
        })
        .unwrap()
}
fn gem(input: &mut SchemaPackageInput) -> &mut GemSchema {
    input
        .definitions
        .iter_mut()
        .find_map(|d| match d {
            DefinitionDescriptor::Gem(DefinitionEntry {
                schema: SchemaState::Known(s),
                ..
            }) => Some(s),
            _ => None,
        })
        .unwrap()
}
fn reject(input: SchemaPackageInput, kind: SchemaPackageErrorKind) {
    match OwnedDefinitionSchemaPackage::new(input, OwnedSchemaLimits::default()) {
        Err(SchemaPackageError::Invalid { kind: actual, path }) => {
            assert_eq!(actual, kind, "at {path}");
            assert!(!path.is_empty());
        }
        other => panic!("expected {kind:?}, received {other:?}"),
    }
}

#[test]
fn every_descriptor_family_loads_and_roundtrips_as_a_source_independent_index() {
    let package = package(input());
    package.identity().validate().unwrap();
    assert_eq!(package.identity().game, "authored-game");
    assert_eq!(package.identity().release, "release-a");
    assert_eq!(
        package.identity().schema_version,
        OWNED_SCHEMA_PACKAGE_VERSION
    );
    assert_eq!(package.identity().semantics_version, "schema-semantics-1");
    assert_eq!(
        package
            .input()
            .definitions
            .iter()
            .map(DefinitionDescriptor::address)
            .map(|a| a.kind())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        24
    );
    assert_eq!(
        package
            .input()
            .slots
            .iter()
            .map(SlotDescriptor::address)
            .map(|a| a.kind())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        6
    );
    assert!(matches!(
        package.definition(&id::<GemDefinition>("gem")),
        SchemaLookup::Known(_)
    ));
    assert!(matches!(
        package.slot(&declared::<ActorSlotDefinition>(
            item_owner(),
            "owned-actor"
        )),
        SchemaLookup::Known(_)
    ));
    assert!(matches!(
        package.definition(&id::<GemDefinition>("missing")),
        SchemaLookup::Missing
    ));
    let foreign = GemDefId::parse(
        GameVersionNamespace::new("other-game", "v1").unwrap(),
        "gem",
    )
    .unwrap();
    assert!(matches!(
        package.definition(&foreign),
        SchemaLookup::NamespaceMismatch
    ));
    let limits = OwnedSchemaLimits::default();
    let bytes = encode_schema_package(&package, limits).unwrap();
    let decoded = decode_schema_package(&bytes, limits).unwrap();
    assert_eq!(decoded.input(), package.input());
    assert_eq!(decoded.identity(), package.identity());
    assert_eq!(encode_schema_package(&decoded, limits).unwrap(), bytes);
}

fn reverse_arrays(value: &mut Value) {
    match value {
        Value::Array(values) => {
            for value in values.iter_mut() {
                reverse_arrays(value);
            }
            values.reverse();
        }
        Value::Object(values) => {
            for value in values.values_mut() {
                reverse_arrays(value);
            }
        }
        _ => {}
    }
}
#[test]
fn unordered_permutations_preserve_identity_but_changed_content_does_not() {
    let original = input();
    let mut permuted = serde_json::to_value(&original).unwrap();
    reverse_arrays(&mut permuted);
    let first = package(original.clone());
    let second = package(serde_json::from_value(permuted).unwrap());
    assert_eq!(first.input(), second.input());
    assert_eq!(first.identity(), second.identity());
    let mut changed = original;
    gem(&mut changed).level.maximum = BoundedInteger::new(31).unwrap();
    let changed = package(changed);
    assert_ne!(first.identity(), changed.identity());
    let SchemaLookup::Known(before) = first.definition(&id::<GemDefinition>("gem")) else {
        panic!("missing original gem")
    };
    let SchemaLookup::Known(after) = changed.definition(&id::<GemDefinition>("gem")) else {
        panic!("missing changed gem")
    };
    assert_eq!(before.level.maximum.get(), 30);
    assert_eq!(after.level.maximum.get(), 31);
}

#[test]
fn identity_only_targets_and_partial_registry_membership_remain_explicit() {
    let mut input = input();
    for descriptor in &mut input.definitions {
        if let DefinitionDescriptor::Skill(entry) = descriptor {
            entry.schema = SchemaState::Unmapped {
                gaps: vec![gap(SchemaSubject::Definition(entry.id.address()))],
            };
        }
    }
    let item_gap = gap(SchemaSubject::Definition(DefinitionAddress::ItemTemplate(
        id("item"),
    )));
    item(&mut input).declarations.choices = DeclaredSet::partial(vec![], vec![item_gap.clone()]);
    gem(&mut input).skills = DeclaredSet::partial(vec![], vec![item_gap]);
    let package = package(input);
    assert!(
        matches!(package.definition(&id::<SkillDefinition>("skill")), SchemaLookup::Unmapped(gaps) if gaps.len() == 1)
    );
    let SchemaLookup::Known(item) = package.definition(&id::<ItemTemplateDefinition>("item"))
    else {
        panic!("missing item schema")
    };
    assert!(item.declarations.choices.members.is_empty());
    assert!(!item.declarations.choices.is_complete());
    // An existing slot entry does not make its partial owner list complete.
    assert!(matches!(
        package.slot(&declared::<ChoiceSlotDefinition>(
            item_owner(),
            "action-choice"
        )),
        SchemaLookup::Known(_)
    ));
    let decoded = decode_schema_package(
        &encode_schema_package(&package, OwnedSchemaLimits::default()).unwrap(),
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    assert_eq!(decoded.input(), package.input());
}

#[test]
fn duplicate_addresses_members_and_foreign_namespaces_cannot_acquire_authority() {
    let mut duplicate = input();
    duplicate.definitions.push(duplicate.definitions[0].clone());
    reject(duplicate, SchemaPackageErrorKind::DuplicateAddress);
    let mut duplicate = input();
    duplicate.slots.push(duplicate.slots[0].clone());
    reject(duplicate, SchemaPackageErrorKind::DuplicateAddress);
    let mut duplicate = input();
    gem(&mut duplicate).roles.push(AuthoredGemRole::SkillUse);
    reject(duplicate, SchemaPackageErrorKind::DuplicateMember);
    let mut duplicate = input();
    item(&mut duplicate).modifiers.members.push(id("modifier"));
    reject(duplicate, SchemaPackageErrorKind::DuplicateMember);
    let mut foreign = input();
    item(&mut foreign).modifiers.members = vec![
        ModifierDefId::parse(
            GameVersionNamespace::new("other-game", "v1").unwrap(),
            "modifier",
        )
        .unwrap(),
    ];
    reject(foreign, SchemaPackageErrorKind::ForeignNamespace);
}

#[test]
fn every_explicit_target_and_gap_subject_requires_registered_identity() {
    let mut missing = input();
    missing
        .definitions
        .retain(|d| d.address() != DefinitionAddress::Unit(id("ratio")));
    reject(missing, SchemaPackageErrorKind::MissingDefinition);
    let mut missing = input();
    missing
        .slots
        .retain(|s| !matches!(s, SlotDescriptor::Actor(_)));
    reject(missing, SchemaPackageErrorKind::MissingSlot);
    let mut missing = input();
    gem(&mut missing).skills = DeclaredSet::partial(
        vec![],
        vec![gap(SchemaSubject::Definition(DefinitionAddress::Skill(
            id("unregistered"),
        )))],
    );
    reject(missing, SchemaPackageErrorKind::MissingDefinition);
    let mut empty = input();
    gem(&mut empty).skills = DeclaredSet::partial(vec![], vec![]);
    reject(empty, SchemaPackageErrorKind::EmptyGapEvidence);
    let mut empty = input();
    if let DefinitionDescriptor::Class(entry) = &mut empty.definitions[0] {
        entry.schema = SchemaState::Unmapped { gaps: vec![] };
    }
    reject(empty, SchemaPackageErrorKind::EmptyGapEvidence);
}

#[test]
fn direct_declarations_sites_and_socket_owners_must_agree() {
    let mut wrong = input();
    item(&mut wrong).declarations.parameters.members =
        vec![declared(SlotOwnerDefId::Gem(id("gem")), "gem-variant")];
    reject(wrong, SchemaPackageErrorKind::WrongDeclaration);
    let mut absent = input();
    item(&mut absent).declarations.outputs = DeclaredSet::complete(vec![]);
    reject(absent, SchemaPackageErrorKind::UndeclaredSlot);
    let mut wrong = input();
    if let SlotDescriptor::Parameter(DefinitionEntry {
        schema: SchemaState::Known(schema),
        ..
    }) = &mut wrong.slots[0]
    {
        schema.sites = vec![ParameterSite::GemParameter];
    }
    reject(wrong, SchemaPackageErrorKind::WrongParameterSite);
    let mut wrong = input();
    item(&mut wrong)
        .declarations
        .sockets
        .members
        .push(id("passive-socket"));
    reject(wrong, SchemaPackageErrorKind::WrongDeclaration);
    let mut wrong = input();
    for descriptor in &mut wrong.definitions {
        if let DefinitionDescriptor::SocketSlot(DefinitionEntry {
            schema: SchemaState::Known(schema),
            ..
        }) = descriptor
            && matches!(schema.owner, SlotOwnerDefId::ItemTemplate(_))
        {
            schema.kind = SocketKind::Passive;
        }
    }
    reject(wrong, SchemaPackageErrorKind::WrongSocketOwner);
}

#[test]
fn ranges_use_ordered_endpoints_and_exact_units_not_equal_dimensions() {
    let mut reversed = input();
    gem(&mut reversed).level = range(30, 1);
    reject(reversed, SchemaPackageErrorKind::ReversedRange);
    let mut reversed = input();
    for d in &mut reversed.definitions {
        if let DefinitionDescriptor::Quality(DefinitionEntry {
            schema: SchemaState::Known(schema),
            ..
        }) = d
        {
            schema.amount = quantity(20.0, 0.0, "ratio");
        }
    }
    reject(reversed, SchemaPackageErrorKind::ReversedRange);
    let mut different_unit = input();
    for d in &mut different_unit.definitions {
        if let DefinitionDescriptor::Quality(DefinitionEntry {
            schema: SchemaState::Known(schema),
            ..
        }) = d
        {
            schema.amount.maximum = FiniteQuantity::new(20.0, id("other-ratio")).unwrap();
        }
    }
    reject(different_unit, SchemaPackageErrorKind::UnitMismatch);
}

#[test]
fn known_empty_scopes_and_potential_cycles_are_not_game_legality_or_activation() {
    let mut raw = input();
    gem(&mut raw).roles.clear();
    if let SlotDescriptor::Parameter(DefinitionEntry {
        schema: SchemaState::Known(schema),
        ..
    }) = &mut raw.slots[0]
    {
        schema.sites.clear();
    }
    let package = package(raw);
    let SchemaLookup::Known(gem) = package.definition(&id::<GemDefinition>("gem")) else {
        panic!("missing gem")
    };
    assert!(gem.roles.is_empty());
    let SchemaLookup::Known(parameter) = package.slot(&declared::<ParameterSlotDefinition>(
        item_owner(),
        "item-state",
    )) else {
        panic!("missing parameter")
    };
    assert!(parameter.sites.is_empty());
    // The source-independent fixture deliberately has a node self-edge and an
    // actor/output relationship cycle. A schema index does not execute either.
    let SchemaLookup::Known(node) = package.definition(&id::<PassiveNodeDefinition>("node")) else {
        panic!("missing node")
    };
    assert_eq!(
        node.adjacent.members,
        vec![id::<PassiveNodeDefinition>("node")]
    );
}

#[test]
fn wire_rejects_unknown_duplicate_missing_and_wrong_kind_fields() {
    let raw = input();
    let limits = OwnedSchemaLimits::default();
    let wire = serde_json::to_value(&raw).unwrap();
    for pointer in ["", "/definitions/0/value/schema/value"] {
        let mut extra = wire.clone();
        extra
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unknown_semantic_field".into(), json!(true));
        assert!(decode_schema_package(&serde_json::to_vec(&extra).unwrap(), limits).is_err());
    }
    let mut missing = wire.clone();
    missing.as_object_mut().unwrap().remove("slots");
    assert!(decode_schema_package(&serde_json::to_vec(&missing).unwrap(), limits).is_err());
    let mut wrong = wire.clone();
    wrong["definitions"][0]["value"]["id"]["kind"] = json!("gem");
    assert!(decode_schema_package(&serde_json::to_vec(&wrong).unwrap(), limits).is_err());
    let encoded = serde_json::to_string(&raw).unwrap();
    let duplicated = encoded.replacen(
        &format!("\"schema_version\":{OWNED_SCHEMA_PACKAGE_VERSION}"),
        &format!("\"schema_version\":{OWNED_SCHEMA_PACKAGE_VERSION},\"schema_version\":{OWNED_SCHEMA_PACKAGE_VERSION}"),
        1,
    );
    assert_ne!(duplicated, encoded);
    assert!(decode_schema_package(duplicated.as_bytes(), limits).is_err());
    let overflow = encoded.replacen("\"value\":0.0", "\"value\":1e999", 1);
    assert_ne!(overflow, encoded);
    assert!(decode_schema_package(overflow.as_bytes(), limits).is_err());
    let mut version = raw;
    version.schema_version = OWNED_SCHEMA_PACKAGE_VERSION + 1;
    assert!(matches!(
        OwnedDefinitionSchemaPackage::new(version, limits),
        Err(SchemaPackageError::UnsupportedVersion(v)) if v == OWNED_SCHEMA_PACKAGE_VERSION + 1
    ));
}

// Independently count the vector entries visible in the wire schema. This catches
// a future DTO vector omitted from the loader's aggregate resource accounting.
fn array_stats(value: &Value) -> (usize, usize) {
    match value {
        Value::Array(values) => values.iter().map(array_stats).fold(
            (values.len(), values.len()),
            |(sum, maximum), (child_sum, child_max)| (sum + child_sum, maximum.max(child_max)),
        ),
        Value::Object(values) => values
            .values()
            .map(array_stats)
            .fold((0, 0), |(sum, maximum), (child_sum, child_max)| {
                (sum + child_sum, maximum.max(child_max))
            }),
        _ => (0, 0),
    }
}
#[test]
fn bytes_total_entries_and_each_collection_obey_exact_tightened_limits() {
    let raw = input();
    let (entries, collection) = array_stats(&serde_json::to_value(&raw).unwrap());
    let package = package(raw.clone());
    let bytes = encode_schema_package(&package, OwnedSchemaLimits::default()).unwrap();
    let exact = OwnedSchemaLimits {
        max_entries: entries,
        max_collection_entries: collection,
        max_wire_bytes: bytes.len(),
    };
    assert!(OwnedDefinitionSchemaPackage::new(raw.clone(), exact).is_ok());
    assert!(decode_schema_package(&bytes, exact).is_ok());
    assert_eq!(encode_schema_package(&package, exact).unwrap(), bytes);
    for below in [
        OwnedSchemaLimits {
            max_entries: entries - 1,
            ..exact
        },
        OwnedSchemaLimits {
            max_collection_entries: collection - 1,
            ..exact
        },
        OwnedSchemaLimits {
            max_wire_bytes: bytes.len() - 1,
            ..exact
        },
    ] {
        assert!(OwnedDefinitionSchemaPackage::new(raw.clone(), below).is_err());
        assert!(decode_schema_package(&bytes, below).is_err());
        assert!(encode_schema_package(&package, below).is_err());
    }
    for invalid in [
        OwnedSchemaLimits {
            max_entries: 0,
            ..exact
        },
        OwnedSchemaLimits {
            max_wire_bytes: HARD_SCHEMA_MAX_WIRE_BYTES + 1,
            ..exact
        },
    ] {
        assert!(OwnedDefinitionSchemaPackage::new(raw.clone(), invalid).is_err());
        assert!(decode_schema_package(&bytes, invalid).is_err());
        assert!(encode_schema_package(&package, invalid).is_err());
    }
    let raised = OwnedSchemaLimits {
        max_entries: DEFAULT_SCHEMA_MAX_ENTRIES + 1,
        max_collection_entries: DEFAULT_SCHEMA_MAX_COLLECTION_ENTRIES + 1,
        max_wire_bytes: DEFAULT_SCHEMA_MAX_WIRE_BYTES + 1,
    };
    assert!(OwnedDefinitionSchemaPackage::new(raw, raised).is_ok());
    assert!(decode_schema_package(&bytes, raised).is_ok());
    assert!(encode_schema_package(&package, raised).is_ok());
}

#[test]
fn large_complete_owner_sets_preserve_membership_after_canonical_sorting() {
    let mut raw = input();
    for ordinal in (0..257).rev() {
        let slot =
            declared::<ParameterSlotDefinition>(item_owner(), &format!("extra-{ordinal:04}"));
        item(&mut raw)
            .declarations
            .parameters
            .members
            .push(slot.clone());
        raw.slots.push(SlotDescriptor::Parameter(known(
            slot,
            ParameterSlotSchema {
                value: ValueSchema::Boolean,
                presence: SlotPresence::OptionalOnce,
                sites: vec![ParameterSite::ItemParameter],
            },
        )));
    }
    let package = package(raw);
    for ordinal in [0, 128, 256] {
        let slot =
            declared::<ParameterSlotDefinition>(item_owner(), &format!("extra-{ordinal:04}"));
        assert!(matches!(package.slot(&slot), SchemaLookup::Known(_)));
    }
    let mut missing = package.input().clone();
    let middle = declared::<ParameterSlotDefinition>(item_owner(), "extra-0128");
    item(&mut missing)
        .declarations
        .parameters
        .members
        .retain(|slot| slot != &middle);
    reject(missing, SchemaPackageErrorKind::UndeclaredSlot);
}

fn computed_input() -> SchemaPackageInput {
    SchemaPackageInput {
        schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
        namespace: ns(),
        release: key("computed"),
        semantics_version: key("declarations-only"),
        slots: vec![],
        definitions: vec![
            DefinitionDescriptor::Unit(known(
                id("exact-unit"),
                UnitSchema {
                    dimension: UnitDimension::Count,
                },
            )),
            DefinitionDescriptor::Stat(known(
                id("computed-stat"),
                StatSchema {
                    value: ComputedValueType::Quantity {
                        unit: id("exact-unit"),
                    },
                    targets: vec![RuleEntityKind::Actor, RuleEntityKind::EquipmentUse],
                },
            )),
            DefinitionDescriptor::Capability(known(
                id("computed-capability"),
                CapabilitySchema {
                    targets: vec![RuleEntityKind::Action, RuleEntityKind::Actor],
                },
            )),
        ],
    }
}
fn computed_stat(input: &mut SchemaPackageInput) -> &mut StatSchema {
    let DefinitionDescriptor::Stat(entry) = &mut input.definitions[1] else {
        unreachable!()
    };
    let SchemaState::Known(schema) = &mut entry.schema else {
        unreachable!()
    };
    schema
}
fn computed_capability(input: &mut SchemaPackageInput) -> &mut CapabilitySchema {
    let DefinitionDescriptor::Capability(entry) = &mut input.definitions[2] else {
        unreachable!()
    };
    let SchemaState::Known(schema) = &mut entry.schema else {
        unreachable!()
    };
    schema
}
#[test]
fn computed_schema_validates_exact_unit_closure_targets_and_limits() {
    let first = package(computed_input());
    let mut shuffled = computed_input();
    computed_stat(&mut shuffled).targets.reverse();
    computed_capability(&mut shuffled).targets.reverse();
    assert_eq!(first.identity(), package(shuffled).identity());
    let bytes = encode_schema_package(&first, OwnedSchemaLimits::default()).unwrap();
    assert_eq!(
        decode_schema_package(&bytes, OwnedSchemaLimits::default())
            .unwrap()
            .input(),
        first.input()
    );
    for capability in [false, true] {
        let mut duplicate = computed_input();
        if capability {
            computed_capability(&mut duplicate)
                .targets
                .push(RuleEntityKind::Actor);
        } else {
            computed_stat(&mut duplicate)
                .targets
                .push(RuleEntityKind::Actor);
        }
        reject(duplicate, SchemaPackageErrorKind::DuplicateMember);
    }
    let mut missing = computed_input();
    computed_stat(&mut missing).value = ComputedValueType::Quantity {
        unit: id("missing-unit"),
    };
    reject(missing, SchemaPackageErrorKind::MissingDefinition);
    let mut foreign = computed_input();
    computed_stat(&mut foreign).value = ComputedValueType::Quantity {
        unit: UnitDefId::parse(
            GameVersionNamespace::new("foreign", "v1").unwrap(),
            "exact-unit",
        )
        .unwrap(),
    };
    reject(foreign, SchemaPackageErrorKind::ForeignNamespace);
    // Identity closure is independent of conversion coverage of the unit.
    let mut unresolved = computed_input();
    let DefinitionDescriptor::Unit(entry) = &mut unresolved.definitions[0] else {
        unreachable!()
    };
    entry.schema = SchemaState::Unmapped {
        gaps: vec![gap(SchemaSubject::Definition(entry.id.address()))],
    };
    let unresolved = package(unresolved);
    assert!(matches!(
        unresolved.definition(&id::<UnitDefinition>("exact-unit")),
        SchemaLookup::Unmapped(_)
    ));
    // Closed empty targets mean no targets; this layer never invents applicability.
    let mut empty = computed_input();
    computed_stat(&mut empty).targets.clear();
    computed_capability(&mut empty).targets.clear();
    let empty = package(empty);
    let SchemaLookup::Known(stat) = empty.definition(&id::<StatDefinition>("computed-stat")) else {
        unreachable!()
    };
    assert!(stat.targets.is_empty());
    let mut bounded = computed_input();
    computed_capability(&mut bounded).targets = vec![
        RuleEntityKind::Actor,
        RuleEntityKind::Action,
        RuleEntityKind::EquipmentUse,
        RuleEntityKind::Enemy,
        RuleEntityKind::Environment,
    ];
    let limits = OwnedSchemaLimits {
        max_collection_entries: 4,
        ..OwnedSchemaLimits::default()
    };
    assert!(matches!(OwnedDefinitionSchemaPackage::new(bounded, limits),
        Err(SchemaPackageError::Invalid { path, kind: SchemaPackageErrorKind::LimitExceeded }) if path.ends_with(".targets")));
}
