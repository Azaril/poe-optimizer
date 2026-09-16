//! Data-only recipe additions preserve prior facts, identities and coverage.
use poe_optimizer_core::{
    owned_build::{DeclaredSlot, ParameterValue},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::{owned_mapping::*, owned_recipe::*, owned_recipe_extension::*};
use std::path::Path;
#[allow(dead_code)]
#[path = "support/owned_compact_fixture.rs"]
mod fixture;

fn key(v: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(v).unwrap()
}
fn partial<T>(subject: &SchemaSubject, facet: SchemaFacet) -> DeclaredSet<T> {
    DeclaredSet::partial(
        vec![],
        vec![SchemaGap {
            subject: subject.clone(),
            facet,
            code: key("not-reviewed"),
        }],
    )
}
fn ports(subject: &SchemaSubject) -> DeclaredSlots {
    DeclaredSlots {
        parameters: partial(subject, SchemaFacet::InputSchema),
        choices: partial(subject, SchemaFacet::InputSchema),
        grants: partial(subject, SchemaFacet::InputSchema),
        actors: partial(subject, SchemaFacet::InputSchema),
        skill_grants: partial(subject, SchemaFacet::InputSchema),
        outputs: partial(subject, SchemaFacet::InputSchema),
        sockets: partial(subject, SchemaFacet::InputSchema),
    }
}
fn record<T, I>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn stat(id: StatDefId, unit: &UnitDefId) -> DefinitionDescriptor {
    DefinitionDescriptor::Stat(record(
        id,
        StatSchema {
            value: ComputedValueType::Quantity { unit: unit.clone() },
            targets: vec![RuleEntityKind::Actor],
        },
    ))
}
fn program(id: &str, stat: &StatDefId, unit: &UnitDefId, value: f64) -> RuleProgram {
    RuleProgram {
        id: key(id),
        context: RuleEntityKind::Actor,
        reads: vec![],
        nodes: vec![RuleNode {
            id: key("value"),
            expression: RuleExpression::Literal {
                value: ParameterValue::Quantity(FiniteQuantity::new(value, unit.clone()).unwrap()),
            },
        }],
        effects: vec![RuleEffect {
            id: key("derive"),
            when: None,
            effect: RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat: stat.clone(),
                value: key("value"),
            },
        }],
    }
}
fn recipe(base: &StagedOwnedRecipe) -> OwnedRecipeInput {
    OwnedRecipeInput {
        schema_version: OWNED_RECIPE_VERSION,
        registry: base.registry().input().clone(),
        schema: base.schema().input().clone(),
        rules: base.rules().input().clone(),
        routing: base.routing().input().clone(),
    }
}
fn rebind(input: &mut OwnedRecipeInput) {
    let schema =
        OwnedDefinitionSchemaPackage::new(input.schema.clone(), Default::default()).unwrap();
    input.rules.definitions = schema.identity().clone();
    input.routing.definitions = schema.identity().clone();
}
fn empty() -> OwnedRecipeExtension {
    OwnedRecipeExtension {
        schema_version: 1,
        version: key("extension-test"),
        schema: vec![],
        operations_version: None,
        tables: vec![],
        owners: vec![],
        receivers: vec![],
    }
}
fn add_definition(extension: &mut OwnedRecipeExtension, definition: DefinitionDescriptor) {
    extension
        .schema
        .push(SchemaExtensionEntry::Definition(definition));
}
struct Fixture {
    base: StagedOwnedRecipe,
    item: ItemTemplateDefId,
    modifier: ModifierDefId,
    stat: StatDefId,
    missing_stat: StatDefId,
    unit: UnitDefId,
    quality: QualityDefId,
}
impl Fixture {
    fn new() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut input = fixture::input(&root).successor;
        let quality = input
            .schema
            .definitions
            .iter()
            .find_map(|d| {
                if let DefinitionDescriptor::Quality(d) = d {
                    Some(d.id.clone())
                } else {
                    None
                }
            })
            .unwrap();
        let mut registry =
            OwnedIdRegistry::new(input.registry.clone(), Default::default()).unwrap();
        let item: ItemTemplateDefId = registry.allocate_definition().unwrap();
        let subject = SchemaSubject::Definition(item.address());
        input
            .schema
            .definitions
            .push(DefinitionDescriptor::ItemTemplate(record(
                item.clone(),
                ItemTemplateSchema {
                    item_level: IntegerRange {
                        minimum: BoundedInteger::new(0).unwrap(),
                        maximum: BoundedInteger::new(100).unwrap(),
                    },
                    equipment_slots: partial(&subject, SchemaFacet::InputSchema),
                    socket_destinations: partial(&subject, SchemaFacet::InputSchema),
                    modifiers: partial(&subject, SchemaFacet::InputSchema),
                    quality: QualityUseSchema {
                        presence: QualityPresence::Optional,
                        allowed_kinds: partial(&subject, SchemaFacet::InputSchema),
                    },
                    declarations: ports(&subject),
                },
            )));
        input.rules.owners.push(DefinitionRules {
            owner: subject.clone(),
            programs: partial(&subject, SchemaFacet::GameRules),
        });
        let unit: UnitDefId = registry.allocate_definition().unwrap();
        input
            .schema
            .definitions
            .push(DefinitionDescriptor::Unit(record(
                unit.clone(),
                UnitSchema {
                    dimension: UnitDimension::Damage,
                },
            )));
        let stat: StatDefId = registry.allocate_definition().unwrap();
        input
            .schema
            .definitions
            .push(self::stat(stat.clone(), &unit));
        let subject = SchemaSubject::Definition(stat.address());
        let mut programs = partial(&subject, SchemaFacet::GameRules);
        programs
            .members
            .push(program("base-number", &stat, &unit, 5.0));
        input.rules.owners.push(DefinitionRules {
            owner: subject,
            programs,
        });
        let missing_stat: StatDefId = registry.allocate_definition().unwrap();
        input
            .schema
            .definitions
            .push(self::stat(missing_stat.clone(), &unit));
        let modifier: ModifierDefId = registry.allocate_definition().unwrap();
        let subject = SchemaSubject::Definition(modifier.address());
        input
            .schema
            .definitions
            .push(DefinitionDescriptor::Modifier(record(
                modifier.clone(),
                ModifierSchema {
                    declarations: ports(&subject),
                },
            )));
        input.rules.owners.push(DefinitionRules {
            owner: subject.clone(),
            programs: partial(&subject, SchemaFacet::GameRules),
        });
        input.registry = registry.input().clone();
        rebind(&mut input);
        Self {
            base: assemble_owned_recipe(input, Default::default()).unwrap(),
            item,
            modifier,
            stat,
            missing_stat,
            unit,
            quality,
        }
    }
    fn extend(
        &self,
        extension: &OwnedRecipeExtension,
    ) -> Result<StagedRecipeExtension, RecipeExtensionError> {
        extend_owned_recipe(&self.base, extension, Default::default())
    }
    fn item(&self) -> DefinitionDescriptor {
        self.base
            .schema()
            .lookup_definition(&self.item.address())
            .unwrap()
            .clone()
    }
    fn owner(&self) -> DefinitionRules {
        self.base
            .rules()
            .input()
            .owners
            .iter()
            .find(|r| r.owner == SchemaSubject::Definition(self.stat.address()))
            .unwrap()
            .clone()
    }
    fn membership(&self) -> OwnedRecipeExtension {
        let mut extension = empty();
        let mut item = self.item();
        let DefinitionDescriptor::ItemTemplate(row) = &mut item else {
            unreachable!()
        };
        let SchemaState::Known(s) = &mut row.schema else {
            unreachable!()
        };
        s.modifiers.members.push(self.modifier.clone());
        s.quality.allowed_kinds.members.push(self.quality.clone());
        add_definition(&mut extension, item);
        let mut modifier = self
            .base
            .schema()
            .lookup_definition(&self.modifier.address())
            .unwrap()
            .clone();
        let mut registry = self.base.registry().clone();
        let slot: DeclaredSlot<ParameterSlotDefId> = registry
            .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Modifier(
                self.modifier.clone(),
            ))
            .unwrap();
        let DefinitionDescriptor::Modifier(row) = &mut modifier else {
            unreachable!()
        };
        let SchemaState::Known(s) = &mut row.schema else {
            unreachable!()
        };
        s.declarations.parameters.members.push(slot.clone());
        add_definition(&mut extension, modifier);
        extension
            .schema
            .push(SchemaExtensionEntry::Slot(SlotDescriptor::Parameter(
                record(
                    slot,
                    ParameterSlotSchema {
                        value: ValueSchema::Boolean,
                        presence: SlotPresence::RequiredOnce,
                        sites: vec![ParameterSite::ModifierRoll],
                    },
                ),
            )));
        extension
    }
    fn receiver(&self) -> OwnedRecipeExtension {
        let mut registry = self.base.registry().clone();
        let stat: StatDefId = registry.allocate_definition().unwrap();
        let mut extension = empty();
        add_definition(&mut extension, self::stat(stat.clone(), &self.unit));
        extension.owners.push(DefinitionRules {
            owner: SchemaSubject::Definition(stat.address()),
            programs: DeclaredSet::complete(vec![program(
                "received-number",
                &stat,
                &self.unit,
                8.0,
            )]),
        });
        extension.receivers.push(ActorStatReceiver {
            id: key("new-stat-receiver"),
            stat,
            program: key("received-number"),
            targets: vec![ActorReceiverTarget::Player],
        });
        extension
    }
}

#[test]
fn empty_extension_preserves_all_canonical_facts_and_versions() {
    let f = Fixture::new();
    let result = f.extend(&empty()).unwrap();
    assert_eq!(result.successor, recipe(&f.base));
    assert!(result.refinement.is_none());
    assert_eq!(result.receipt.allocated_entries, 0);
    assert_eq!(result.receipt.appended_programs, 0);
    assert_eq!(
        result.receipt.before_definitions,
        result.receipt.after_definitions
    );
    assert_eq!(
        result.successor.rules.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_V6
    );
}

#[test]
fn interleaved_typed_definition_and_slot_allocations_require_exact_registry_order() {
    let f = Fixture::new();
    let mut registry = f.base.registry().clone();
    let modifier: ModifierDefId = registry.allocate_definition().unwrap();
    let slot = registry
        .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Modifier(modifier.clone()))
        .unwrap();
    let unit: UnitDefId = registry.allocate_definition().unwrap();
    let stat: StatDefId = registry.allocate_definition().unwrap();
    let subject = SchemaSubject::Definition(modifier.address());
    let mut declarations = ports(&subject);
    declarations.parameters.members.push(slot.clone());
    let mut extension = empty();
    add_definition(
        &mut extension,
        DefinitionDescriptor::Modifier(record(modifier, ModifierSchema { declarations })),
    );
    extension
        .schema
        .push(SchemaExtensionEntry::Slot(SlotDescriptor::Parameter(
            record(
                slot,
                ParameterSlotSchema {
                    value: ValueSchema::Boolean,
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::ModifierRoll],
                },
            ),
        )));
    add_definition(
        &mut extension,
        DefinitionDescriptor::Unit(record(
            unit.clone(),
            UnitSchema {
                dimension: UnitDimension::Damage,
            },
        )),
    );
    add_definition(&mut extension, self::stat(stat, &unit));
    let result = f.extend(&extension).unwrap();
    assert_eq!(result.receipt.allocated_entries, 4);
    assert_eq!(result.successor.registry, *registry.input());
    assert!(result.refinement.is_none());
    let mut wrong = extension.clone();
    wrong.schema.swap(0, 1);
    assert!(f.extend(&wrong).is_err());
    let mut wrong = extension.clone();
    wrong.schema.remove(0);
    assert!(f.extend(&wrong).is_err());
    let mut wrong = extension.clone();
    wrong.schema.push(extension.schema.last().unwrap().clone());
    assert!(f.extend(&wrong).is_err());
    assert_eq!(recipe(&f.base).registry, *f.base.registry().input());
}

#[test]
fn partial_membership_additions_emit_v3_refinement_and_repeat_is_a_fixed_point() {
    let f = Fixture::new();
    let extension = f.membership();
    let first = f.extend(&extension).unwrap();
    assert_eq!(first.receipt.allocated_entries, 1);
    assert_eq!(first.receipt.refined_subjects, 2);
    let proof = first.refinement.as_ref().unwrap();
    assert_eq!(proof.schema_version, 3);
    assert_eq!(proof.before, *f.base.schema().identity());
    assert_eq!(proof.after, first.receipt.after_definitions);
    let next = assemble_owned_recipe(first.successor.clone(), Default::default()).unwrap();
    let second = extend_owned_recipe(&next, &extension, Default::default()).unwrap();
    assert_eq!(second.successor, first.successor);
    assert!(second.refinement.is_none());
    assert_eq!(second.receipt.allocated_entries, 0);
    assert_eq!(second.receipt.refined_subjects, 0);
    for entry in &extension.schema {
        if let SchemaExtensionEntry::Definition(DefinitionDescriptor::ItemTemplate(row)) = entry {
            let SchemaState::Known(s) = &row.schema else {
                unreachable!()
            };
            assert!(!s.modifiers.is_complete());
            assert!(!s.quality.allowed_kinds.is_complete());
        }
    }
}

#[test]
fn existing_members_scalars_and_closure_evidence_cannot_be_replaced() {
    let f = Fixture::new();
    let good = f.membership();
    let first = f.extend(&good).unwrap();
    let base = assemble_owned_recipe(first.successor, Default::default()).unwrap();
    for case in 0..4 {
        let mut wrong = empty();
        let mut item = base
            .schema()
            .lookup_definition(&f.item.address())
            .unwrap()
            .clone();
        let DefinitionDescriptor::ItemTemplate(row) = &mut item else {
            unreachable!()
        };
        let SchemaState::Known(s) = &mut row.schema else {
            unreachable!()
        };
        match case {
            0 => s.modifiers.members.clear(),
            1 => s.modifiers.closure = SchemaClosure::Complete,
            2 => s.item_level.maximum = BoundedInteger::new(101).unwrap(),
            _ => {
                if let SchemaClosure::Partial { gaps } = &mut s.modifiers.closure {
                    gaps[0].code = key("new-claim");
                }
            }
        }
        add_definition(&mut wrong, item);
        assert!(
            extend_owned_recipe(&base, &wrong, Default::default()).is_err(),
            "case {case}"
        );
    }
    let mut wrong = empty();
    add_definition(
        &mut wrong,
        DefinitionDescriptor::Unit(record(
            f.unit.clone(),
            UnitSchema {
                dimension: UnitDimension::Rate,
            },
        )),
    );
    assert!(f.extend(&wrong).is_err());
    // An existing complete set cannot be extended merely by keeping its closure.
    let mut input = recipe(&f.base);
    for d in &mut input.schema.definitions {
        if let DefinitionDescriptor::ItemTemplate(row) = d
            && row.id == f.item
            && let SchemaState::Known(s) = &mut row.schema
        {
            s.modifiers = DeclaredSet::complete(vec![]);
        }
    }
    rebind(&mut input);
    let base = assemble_owned_recipe(input, Default::default()).unwrap();
    assert!(extend_owned_recipe(&base, &good, Default::default()).is_err());
}

#[test]
fn owner_program_additions_preserve_existing_programs_and_partial_closure() {
    let f = Fixture::new();
    let old = f.owner();
    let mut extension = empty();
    let mut owner = old.clone();
    owner
        .programs
        .members
        .push(program("additional-number", &f.stat, &f.unit, 6.0));
    extension.owners.push(owner);
    let first = f.extend(&extension).unwrap();
    assert_eq!(first.receipt.appended_programs, 1);
    let base = assemble_owned_recipe(first.successor.clone(), Default::default()).unwrap();
    assert_eq!(
        extend_owned_recipe(&base, &extension, Default::default())
            .unwrap()
            .successor,
        first.successor
    );
    for case in 0..4 {
        let mut wrong = empty();
        let mut owner = old.clone();
        match case {
            0 => owner.programs.members[0] = program("base-number", &f.stat, &f.unit, 9.0),
            1 => owner.programs.closure = SchemaClosure::Complete,
            2 => owner
                .programs
                .members
                .push(owner.programs.members[0].clone()),
            _ => {
                wrong.owners.push(owner.clone());
            }
        }
        wrong.owners.push(owner);
        assert!(f.extend(&wrong).is_err(), "case {case}");
    }
    let mut input = recipe(&f.base);
    input
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == old.owner)
        .unwrap()
        .programs
        .closure = SchemaClosure::Complete;
    let complete = assemble_owned_recipe(input, Default::default()).unwrap();
    extension.owners[0].programs.closure = SchemaClosure::Complete;
    assert!(extend_owned_recipe(&complete, &extension, Default::default()).is_err());
}

#[test]
fn table_identity_and_values_are_append_only_and_unit_checked() {
    let f = Fixture::new();
    let mut extension = empty();
    extension.tables.push(IntegerRuleTable {
        id: key("new-table"),
        minimum: BoundedInteger::new(0).unwrap(),
        maximum: BoundedInteger::new(1).unwrap(),
        value_type: ComputedValueType::Integer,
        rows: vec![
            ParameterValue::Integer(BoundedInteger::new(2).unwrap()),
            ParameterValue::Integer(BoundedInteger::new(3).unwrap()),
        ],
    });
    let first = f.extend(&extension).unwrap();
    assert_eq!(first.receipt.appended_tables, 1);
    let base = assemble_owned_recipe(first.successor.clone(), Default::default()).unwrap();
    assert_eq!(
        extend_owned_recipe(&base, &extension, Default::default())
            .unwrap()
            .successor,
        first.successor
    );
    let mut wrong = extension.clone();
    wrong.tables[0].rows[0] = ParameterValue::Integer(BoundedInteger::new(4).unwrap());
    assert!(extend_owned_recipe(&base, &wrong, Default::default()).is_err());
    let mut wrong = extension.clone();
    wrong.tables.push(wrong.tables[0].clone());
    assert!(f.extend(&wrong).is_err());
    let mut wrong = extension;
    wrong.tables[0].rows[0] =
        ParameterValue::Quantity(FiniteQuantity::new(2.0, f.unit.clone()).unwrap());
    assert!(f.extend(&wrong).is_err());
}

#[test]
fn complete_receiver_registry_only_grows_for_newly_allocated_stats() {
    let f = Fixture::new();
    assert!(f.base.rules().input().receivers.is_complete());
    let extension = f.receiver();
    let first = f.extend(&extension).unwrap();
    assert_eq!(first.receipt.appended_receivers, 1);
    assert_eq!(first.receipt.appended_programs, 1);
    assert!(first.successor.rules.receivers.is_complete());
    let base = assemble_owned_recipe(first.successor.clone(), Default::default()).unwrap();
    assert_eq!(
        extend_owned_recipe(&base, &extension, Default::default())
            .unwrap()
            .successor,
        first.successor
    );
    let mut old = empty();
    old.receivers.push(ActorStatReceiver {
        id: key("prior-stat-receiver"),
        stat: f.stat.clone(),
        program: key("base-number"),
        targets: vec![ActorReceiverTarget::Player],
    });
    assert!(f.extend(&old).is_err());
    let mut wrong = extension.clone();
    wrong.receivers[0].targets.clear();
    assert!(extend_owned_recipe(&base, &wrong, Default::default()).is_err());
    let mut wrong = extension.clone();
    wrong.receivers.push(wrong.receivers[0].clone());
    assert!(f.extend(&wrong).is_err());
    let mut wrong = extension.clone();
    let mut competing = wrong.receivers[0].clone();
    competing.id = key("competing-new-stat-receiver");
    wrong.receivers.push(competing);
    assert!(f.extend(&wrong).is_err());
    let other_unit = f
        .base
        .schema()
        .input()
        .definitions
        .iter()
        .find_map(|d| {
            if let DefinitionDescriptor::Unit(row) = d
                && row.id != f.unit
            {
                Some(row.id.clone())
            } else {
                None
            }
        })
        .unwrap();
    let mut wrong = extension.clone();
    wrong.owners[0].programs.members[0].nodes[0].expression = RuleExpression::Literal {
        value: ParameterValue::Quantity(FiniteQuantity::new(8.0, other_unit).unwrap()),
    };
    assert!(f.extend(&wrong).is_err());
    let mut wrong = extension;
    wrong.receivers[0].program = key("absent-program");
    assert!(f.extend(&wrong).is_err());
}

#[test]
fn previously_uncovered_owner_cannot_gain_a_complete_rule_claim() {
    let f = Fixture::new();
    let mut extension = empty();
    let subject = SchemaSubject::Definition(f.missing_stat.address());
    extension.owners.push(DefinitionRules {
        owner: subject.clone(),
        programs: DeclaredSet::complete(vec![program(
            "first-number",
            &f.missing_stat,
            &f.unit,
            4.0,
        )]),
    });
    assert!(f.extend(&extension).is_err());
    let mut programs = partial(&subject, SchemaFacet::GameRules);
    programs.members = extension.owners[0].programs.members.clone();
    extension.owners[0].programs = programs;
    assert_eq!(f.extend(&extension).unwrap().receipt.appended_programs, 1);
}

#[test]
fn operations_upgrade_is_explicit_and_downgrades_and_unknown_versions_reject() {
    let f = Fixture::new();
    let mut extension = empty();
    extension.operations_version = Some(key(OWNED_RULE_OPERATIONS_VERSION));
    let first = f.extend(&extension).unwrap();
    assert_eq!(
        first.successor.rules.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_VERSION
    );
    let base = assemble_owned_recipe(first.successor.clone(), Default::default()).unwrap();
    assert_eq!(
        extend_owned_recipe(&base, &extension, Default::default())
            .unwrap()
            .successor,
        first.successor
    );
    extension.operations_version = Some(key(OWNED_RULE_OPERATIONS_V6));
    assert!(extend_owned_recipe(&base, &extension, Default::default()).is_err());
    extension.operations_version = Some(key("future-unknown-ops"));
    assert!(f.extend(&extension).is_err());
}

#[test]
fn bounded_extension_rejects_bad_versions_bytes_work_and_unknown_wire_fields() {
    let f = Fixture::new();
    let mut extension = empty();
    extension.schema_version = 2;
    assert!(f.extend(&extension).is_err());
    extension.schema_version = 1;
    for limits in [
        RecipeExtensionLimits {
            max_wire_bytes: 1,
            ..Default::default()
        },
        RecipeExtensionLimits {
            max_work: 1,
            ..Default::default()
        },
        RecipeExtensionLimits {
            max_work: 0,
            ..Default::default()
        },
        RecipeExtensionLimits {
            max_wire_bytes: RecipeExtensionLimits::default().max_wire_bytes + 1,
            ..Default::default()
        },
    ] {
        assert!(extend_owned_recipe(&f.base, &extension, limits).is_err());
    }
    let mut value = serde_json::to_value(&extension).unwrap();
    value["unreviewed"] = true.into();
    assert!(serde_json::from_value::<OwnedRecipeExtension>(value).is_err());
}

#[test]
fn reversed_members_and_receiver_targets_canonicalize_to_a_repeatable_extension() {
    let f = Fixture::new();
    let mut extension = f.receiver();
    let mut item = f.item();
    let DefinitionDescriptor::ItemTemplate(row) = &mut item else {
        unreachable!()
    };
    let SchemaState::Known(schema) = &mut row.schema else {
        unreachable!()
    };
    let mut modifiers: Vec<_> = f
        .base
        .schema()
        .input()
        .definitions
        .iter()
        .filter_map(|d| {
            if let DefinitionDescriptor::Modifier(row) = d {
                Some(row.id.clone())
            } else {
                None
            }
        })
        .take(2)
        .collect();
    assert_eq!(modifiers.len(), 2);
    modifiers.sort();
    modifiers.reverse();
    schema.modifiers.members = modifiers;
    extension
        .schema
        .insert(0, SchemaExtensionEntry::Definition(item));
    let slot = f
        .base
        .schema()
        .input()
        .slots
        .iter()
        .find_map(|d| {
            if let SlotDescriptor::Actor(row) = d {
                Some(row.id.clone())
            } else {
                None
            }
        })
        .unwrap();
    let targets = &mut extension.receivers[0].targets;
    targets.push(ActorReceiverTarget::OwnedSlot { slot });
    targets.sort();
    targets.reverse();
    let first = f.extend(&extension).unwrap();
    assert_eq!(first.receipt.allocated_entries, 1);
    assert_eq!(first.receipt.refined_subjects, 1);
    assert_eq!(first.receipt.appended_receivers, 1);
    let next = assemble_owned_recipe(first.successor.clone(), Default::default()).unwrap();
    let second = extend_owned_recipe(&next, &extension, Default::default()).unwrap();
    assert_eq!(second.successor, first.successor);
    assert!(second.refinement.is_none());
    assert_eq!(second.receipt.allocated_entries, 0);
    assert_eq!(second.receipt.refined_subjects, 0);
    assert_eq!(second.receipt.appended_receivers, 0);
    assert_eq!(second.receipt.appended_programs, 0);
    let SchemaExtensionEntry::Definition(DefinitionDescriptor::ItemTemplate(row)) =
        &mut extension.schema[0]
    else {
        unreachable!()
    };
    let SchemaState::Known(schema) = &mut row.schema else {
        unreachable!()
    };
    schema.modifiers.members.reverse();
    extension.receivers[0].targets.reverse();
    let reordered = extend_owned_recipe(&next, &extension, Default::default()).unwrap();
    assert_eq!(reordered.successor, first.successor);
    assert!(reordered.refinement.is_none());
}
