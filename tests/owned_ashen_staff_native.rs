//! The published Firebolt grant program in an explicitly finite native topology.
//! This transports the rebuilt raw grant level. It does not supply an effective
//! level, quality, action, saved-group activation, or a complete original build.
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;

use poe_optimizer_core::{
    build_identity::*, owned_binding::*, owned_build::*, owned_definitions::*, owned_routing::*,
    owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::{
    owned_routing::OwnedActionRouting,
    owned_schema::{OwnedDefinitionSchemaPackage, SchemaPackageInput},
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::CompiledRulePackage};
use poe_optimizer_import::owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{fs, path::Path, sync::Arc};

fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn typed<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn integer(value: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(value).unwrap())
}
fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
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
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}
fn close(declarations: &mut DeclaredSlots) {
    declarations.parameters.closure = SchemaClosure::Complete;
    declarations.choices.closure = SchemaClosure::Complete;
    declarations.grants.closure = SchemaClosure::Complete;
    declarations.actors.closure = SchemaClosure::Complete;
    declarations.skill_grants.closure = SchemaClosure::Complete;
    declarations.outputs.closure = SchemaClosure::Complete;
    declarations.sockets.closure = SchemaClosure::Complete;
}
fn assignments(values: &Value) -> Vec<ParameterAssignment> {
    assert_eq!(values["completion"], json!({"kind":"complete"}));
    values["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            assert_eq!(row["slot"]["kind"], "known");
            assert_eq!(row["value"]["kind"], "known");
            ParameterAssignment {
                slot: typed(&row["slot"]["value"]),
                value: typed(&row["value"]["value"]),
            }
        })
        .collect()
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Bindings {
    modifier: ModifierDefId,
    level: DeclaredSlot<ParameterSlotDefId>,
    grant: DeclaredSlot<GrantSlotDefId>,
    supply: DeclaredSlot<SkillGrantSlotDefId>,
    skill: SkillDefId,
    raw_level: DeclaredSlot<ParameterSlotDefId>,
    templates: Vec<ItemTemplateDefId>,
}
#[derive(Clone)]
struct Fixture {
    bindings: Bindings,
    schema: SchemaPackageInput,
    rules: RulePackageInput,
    build: BuildInput,
    scenario: ScenarioInput,
    original_definitions: Vec<DefinitionDescriptor>,
    original_slots: Vec<SlotDescriptor>,
    original_owner: DefinitionRules,
}
impl Fixture {
    fn load() -> Self {
        let output = std::path::PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ASHEN_STAFF_OUTPUT")
                .expect("set to the verified Ashen Staff publication output"),
        );
        let endpoint = release::load(&output.join("package"));
        let recipe = &endpoint.input().recipe;
        let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data/owned/poe2/3887ae68/firebolt-item-grant");
        let bindings: Bindings = read(directory.join("bindings.json"));
        let extension: OwnedRecipeExtension = read(directory.join("extension.json"));
        assert_eq!(bindings.templates.len(), 1);
        assert_eq!(extension.owners.len(), 1);
        let original_owner = recipe
            .rules
            .owners
            .iter()
            .find(|owner| owner.owner == SchemaSubject::Definition(bindings.modifier.address()))
            .unwrap()
            .clone();
        assert_eq!(
            original_owner, extension.owners[0],
            "published authored grant body and coverage"
        );
        assert_eq!(original_owner.programs.members.len(), 1);
        assert!(!original_owner.programs.is_complete());
        assert_eq!(
            original_owner.programs.members[0].context,
            RuleEntityKind::EquipmentUse
        );
        assert_eq!(original_owner.programs.members[0].effects.len(), 2);
        let original_definitions: Vec<_> = recipe
            .schema
            .definitions
            .iter()
            .filter(|row| {
                [bindings.modifier.address(), bindings.skill.address()].contains(&row.address())
            })
            .cloned()
            .collect();
        assert_eq!(original_definitions.len(), 2);
        let original_slots: Vec<_> = recipe
            .schema
            .slots
            .iter()
            .filter(|row| {
                [
                    SlotOwnerDefId::Modifier(bindings.modifier.clone()),
                    SlotOwnerDefId::Skill(bindings.skill.clone()),
                ]
                .contains(row.address().declaration())
            })
            .cloned()
            .collect();
        assert_eq!(original_slots.len(), 4);
        for row in &extension.schema {
            match row {
                SchemaExtensionEntry::Definition(row) => {
                    assert!(original_definitions.contains(row))
                }
                SchemaExtensionEntry::Slot(row) => assert!(original_slots.contains(row)),
            }
        }

        let draft: Value = read(output.join("original-05/draft.json"));
        let sidecar: Value = read(output.join("original-05/sidecar.json"));
        let selection: Value = read(output.join("original-05-selection.json"));
        // Source identity is the join; an item title never selects the provider.
        let origin = sidecar["origins"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["source"]["ordinal"] == 594)
            .unwrap();
        let links: Vec<_> = origin["links"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["kind"] == "item")
            .collect();
        assert_eq!(links.len(), 1);
        let staff = draft["draft"]["items"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == links[0]["value"])
            .unwrap();
        assert_eq!(
            typed::<ItemTemplateDefId>(&staff["template"]["value"]),
            bindings.templates[0]
        );
        assert_eq!(assignments(&staff["parameters"]).len(), 6);
        assert_eq!(staff["modifiers"]["completion"], json!({"kind":"complete"}));
        let members = staff["modifiers"]["members"].as_array().unwrap();
        assert_eq!(members.len(), 2);
        assert_eq!(
            members
                .iter()
                .map(|row| assignments(&row["rolls"]).len())
                .sum::<usize>(),
            25
        );
        let grant = members
            .iter()
            .find(|row| typed::<ModifierDefId>(&row["definition"]["value"]) == bindings.modifier)
            .unwrap();
        let rolls = assignments(&grant["rolls"]);
        assert_eq!(
            rolls,
            vec![ParameterAssignment {
                slot: bindings.level.clone(),
                value: integer(11)
            }]
        );
        let item_id = typed(&staff["id"]);
        let actual_uses: Vec<_> = draft["draft"]["equipment"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["item"]["kind"] == "known" && row["item"]["value"] == staff["id"])
            .collect();
        assert_eq!(actual_uses.len(), 1);
        let row = actual_uses[0];
        assert_eq!(row["destination"]["kind"], "character_slot");
        assert_eq!(row["destination"]["value"]["kind"], "known");
        assert_eq!(row["scope"]["kind"], "known");
        let destination: EquipmentSlotDefId = typed(&row["destination"]["value"]["value"]);
        let equipment = EquipmentUse {
            id: typed(&row["id"]),
            item: item_id,
            destination: EquipmentDestination::CharacterSlot(destination.clone()),
            scope: typed(&row["scope"]["value"]),
        };
        let active = typed(&selection["build"]["active_weapon_loadout"]);
        assert!(
            matches!(&equipment.scope, LoadoutScope::Selected {loadouts} if loadouts == &[active])
        );
        let namespace = recipe.schema.namespace.clone();
        let class: ClassDefId = DefId::parse(namespace.clone(), "component-class").unwrap();
        let encounter: EncounterDefId =
            DefId::parse(namespace.clone(), "component-encounter").unwrap();
        let level = IntegerRange {
            minimum: BoundedInteger::new(1).unwrap(),
            maximum: BoundedInteger::new(100).unwrap(),
        };
        let mut definitions = original_definitions.clone();
        // Finite test authority: only the raw grant and its declared target exist.
        // No real endpoint closure is modified, and no physical Gem is invented.
        for row in &mut definitions {
            match row {
                DefinitionDescriptor::Modifier(row) => {
                    let SchemaState::Known(s) = &mut row.schema else {
                        panic!()
                    };
                    close(&mut s.declarations);
                }
                DefinitionDescriptor::Skill(row) => {
                    let SchemaState::Known(s) = &mut row.schema else {
                        panic!()
                    };
                    assert!(!s.directly_selectable);
                    close(&mut s.declarations);
                }
                _ => unreachable!(),
            }
        }
        definitions.extend([
            DefinitionDescriptor::Class(known(
                class.clone(),
                ClassSchema {
                    level: level.clone(),
                    ascendancies: DeclaredSet::complete(vec![]),
                    implicit_passives: DeclaredSet::complete(vec![]),
                    declarations: declarations(),
                },
            )),
            DefinitionDescriptor::Encounter(known(
                encounter.clone(),
                EncounterSchema {
                    enemy_level: level.clone(),
                    external_inputs: DeclaredSet::complete(vec![]),
                },
            )),
            DefinitionDescriptor::ItemTemplate(known(
                bindings.templates[0].clone(),
                ItemTemplateSchema {
                    item_level: level,
                    equipment_slots: DeclaredSet::complete(vec![destination.clone()]),
                    socket_destinations: DeclaredSet::complete(vec![]),
                    modifiers: DeclaredSet::complete(vec![bindings.modifier.clone()]),
                    quality: QualityUseSchema {
                        presence: QualityPresence::Forbidden,
                        allowed_kinds: DeclaredSet::complete(vec![]),
                    },
                    declarations: declarations(),
                },
            )),
            recipe
                .schema
                .definitions
                .iter()
                .find(|row| row.address() == destination.address())
                .unwrap()
                .clone(),
        ]);
        let mut slots = original_slots.clone();
        for row in &mut slots {
            if let SlotDescriptor::SkillGrant(row) = row {
                let SchemaState::Known(schema) = &mut row.schema else {
                    panic!()
                };
                assert_eq!(schema.skill, bindings.skill);
                assert!(schema.outputs.members.is_empty());
                schema.outputs.closure = SchemaClosure::Complete;
            }
        }
        let schema = SchemaPackageInput {
            schema_version: recipe.schema.schema_version,
            namespace: namespace.clone(),
            release: key("unpublished-item-grant-component"),
            semantics_version: key("finite-raw-grant-only"),
            definitions,
            slots,
        };
        let checked =
            OwnedDefinitionSchemaPackage::new(schema.clone(), Default::default()).unwrap();
        let mut owner = original_owner.clone();
        owner.programs.closure = SchemaClosure::Complete;
        assert_eq!(owner.programs.members, original_owner.programs.members);
        let mut owners = vec![owner];
        for subject in [
            SchemaSubject::Definition(class.address()),
            SchemaSubject::Definition(encounter.address()),
            SchemaSubject::Definition(bindings.templates[0].address()),
            SchemaSubject::Definition(bindings.skill.address()),
            SchemaSubject::Slot(SlotAddress::Grant(bindings.grant.clone())),
            SchemaSubject::Slot(SlotAddress::SkillGrant(bindings.supply.clone())),
        ] {
            owners.push(DefinitionRules {
                owner: subject,
                programs: DeclaredSet::complete(vec![]),
            });
        }
        let rules = RulePackageInput {
            existing_actor_rules: None,
            contribution_queries: None,
            effect_applications: None,
            schema_version: recipe.rules.schema_version,
            namespace: namespace.clone(),
            release: schema.release.clone(),
            semantics_version: schema.semantics_version.clone(),
            operations_version: recipe.rules.operations_version.clone(),
            definitions: checked.identity().clone(),
            tables: vec![],
            owners,
            receivers: DeclaredSet::complete(vec![]),
        };
        // The complete physical Staff was checked above. Its other modifier and
        // item inputs are outside this deliberately isolated grant component.
        let rolled = RolledModifier {
            id: typed(&grant["id"]),
            definition: bindings.modifier.clone(),
            rolls,
        };
        let build = BuildInput {
            generated_inputs: None,
            allocator: typed(&draft["draft"]["allocator"]),
            revision: typed(&draft["draft"]["revision"]),
            game_version: namespace.clone(),
            character: CharacterSpec {
                class,
                ascendancy: None,
                level: 20,
                rewards: vec![],
            },
            weapon_loadouts: typed(&draft["draft"]["weapon_loadouts"]["members"]),
            active_weapon_loadout: active,
            items: vec![ItemRecord {
                id: item_id,
                template: bindings.templates[0].clone(),
                parameters: vec![],
                item_level: None,
                quality: None,
                modifier_order: vec![rolled.id],
                modifiers: vec![rolled],
            }],
            equipment: vec![equipment],
            gems: vec![],
            allocations: vec![],
            skills: vec![],
            supports: vec![],
            support_origins: Some(vec![]),
            payload_links: vec![],
            choices: vec![],
        };
        let scenario = ScenarioInput {
            game_version: namespace,
            enemy: EnemySpec {
                encounter,
                level: 20,
            },
            assumptions: vec![],
            usage: vec![],
        };
        Self {
            bindings,
            schema,
            rules,
            build,
            scenario,
            original_definitions,
            original_slots,
            original_owner,
        }
    }
    fn request(&self) -> OwnedEvaluationRequest {
        let limits = OwnedInputLimits::default();
        OwnedEvaluationRequest::new(
            BuildSpec::new(self.build.clone(), limits).unwrap(),
            ScenarioSpec::new(self.scenario.clone(), limits).unwrap(),
            QuerySpec::new(
                QueryInput {
                    game_version: self.build.game_version.clone(),
                    requests: vec![],
                },
                limits,
            )
            .unwrap(),
            limits,
        )
        .unwrap()
    }
    fn schema(&self) -> Arc<OwnedDefinitionSchemaPackage> {
        Arc::new(
            OwnedDefinitionSchemaPackage::new(self.schema.clone(), Default::default()).unwrap(),
        )
    }
    fn plan(
        &self,
    ) -> std::result::Result<OwnedEffectPlan<OwnedDefinitionSchemaPackage>, PlanError> {
        let schema = self.schema();
        let mut rules = self.rules.clone();
        rules.definitions = schema.identity().clone();
        let rules = Arc::new(
            CompiledRulePackage::compile(&rules, schema.as_ref(), Default::default()).unwrap(),
        );
        let routing = Arc::new(
            OwnedActionRouting::new(
                ActionRoutingInput {
                    schema_version: OWNED_ACTION_ROUTING_VERSION,
                    namespace: self.schema.namespace.clone(),
                    release: self.schema.release.clone(),
                    definitions: schema.identity().clone(),
                    outputs: vec![],
                },
                schema.as_ref(),
                Default::default(),
            )
            .unwrap(),
        );
        OwnedEffectPlan::compile(
            Arc::new(self.request()),
            schema,
            rules,
            routing,
            Default::default(),
        )
    }
    fn provider(&self, use_index: usize) -> ProviderKey {
        let equipment = &self.build.equipment[use_index];
        let item = self
            .build
            .items
            .iter()
            .find(|row| row.id == equipment.item)
            .unwrap();
        ProviderKey {
            root: ProviderRoot::ItemModifier {
                equipment_use: equipment.id,
                modifier: item.modifiers[0].id,
            },
            grant_path: vec![],
        }
    }
    fn generated(&self, use_index: usize) -> GeneratedSkillKey {
        GeneratedSkillKey {
            provider: self.provider(use_index),
            slot: self.bindings.supply.clone(),
        }
    }
    fn raw_key(&self, use_index: usize) -> PlanValueKey {
        PlanValueKey::SkillParameter {
            skill: Box::new(self.generated(use_index)),
            parameter: self.bindings.raw_level.clone(),
        }
    }
    fn grant_key(&self, use_index: usize) -> PlanValueKey {
        PlanValueKey::Grant {
            provider: self.provider(use_index),
            slot: self.bindings.grant.clone(),
        }
    }
    fn extra_use(&mut self, new_item: bool, active: bool) {
        let mut allocator = InstanceAllocator::from_state(self.build.allocator);
        let mut equipment = self.build.equipment[0].clone();
        equipment.id = allocator.allocate().unwrap();
        if new_item {
            let mut item = self.build.items[0].clone();
            item.id = allocator.allocate().unwrap();
            item.modifiers[0].id = allocator.allocate().unwrap();
            item.modifier_order = vec![item.modifiers[0].id];
            item.modifiers[0].rolls[0].value = integer(20);
            equipment.item = item.id;
            self.build.items.push(item);
        }
        let destination: EquipmentSlotDefId = DefId::parse(
            self.schema.namespace.clone(),
            format!("component-destination-{}", equipment.id.local()),
        )
        .unwrap();
        self.schema
            .definitions
            .push(DefinitionDescriptor::EquipmentSlot(known(
                destination.clone(),
                EquipmentSlotSchema {
                    scope: ScopePolicy::Either,
                },
            )));
        let row = self
            .schema
            .definitions
            .iter_mut()
            .find_map(|row| match row {
                DefinitionDescriptor::ItemTemplate(row) => Some(row),
                _ => None,
            })
            .unwrap();
        let SchemaState::Known(schema) = &mut row.schema else {
            panic!()
        };
        schema.equipment_slots.members.push(destination.clone());
        equipment.destination = EquipmentDestination::CharacterSlot(destination);
        if !active {
            let inactive = *self
                .build
                .weapon_loadouts
                .iter()
                .find(|id| **id != self.build.active_weapon_loadout)
                .unwrap();
            equipment.scope = LoadoutScope::Selected {
                loadouts: vec![inactive],
            };
        }
        self.build.equipment.push(equipment);
        self.build.allocator = allocator.state();
    }
}
fn value<'a>(report: &'a OwnedEffectsReport, key: &PlanValueKey) -> Option<&'a EffectValue> {
    let mut rows = report.values.iter().filter(|row| &row.key == key);
    let row = rows.next();
    assert!(rows.next().is_none(), "one exact occurrence value");
    row.map(|row| &row.value)
}

#[test]
#[ignore = "requires the checked Ashen Staff publication and unchanged Original05 normalization"]
fn published_firebolt_grant_transports_raw_level_with_exact_native_occurrences() {
    let f = Fixture::load();
    assert!(f.build.gems.is_empty() && f.build.skills.is_empty() && f.scenario.usage.is_empty());
    let plan = f.plan().unwrap();
    let mut scratch = plan.new_scratch();
    let report = plan.evaluate(&mut scratch).unwrap();
    assert!(
        report.gaps.is_empty(),
        "finite fixture only: {:?}",
        report.gaps
    );
    assert_eq!(report.effects.len(), 2);
    assert_eq!(
        value(&report, &f.raw_key(0)),
        Some(&EffectValue::Known { value: integer(11) })
    );
    assert_eq!(
        value(&report, &f.grant_key(0)),
        Some(&EffectValue::Known {
            value: ParameterValue::Boolean(true)
        })
    );
    for row in &report.effects {
        assert_eq!(
            row.key.invocation.origin,
            RuleOrigin::Provider {
                provider: f.provider(0)
            }
        );
        assert_eq!(
            row.key.invocation.entity,
            ConcreteEntity::EquipmentUse(f.build.equipment[0].id)
        );
    }
    let resolver =
        OwnedOccurrenceResolver::new(plan.definitions(), plan.request(), Default::default())
            .unwrap();
    let target = SkillTarget::Generated(Box::new(f.generated(0)));
    let resolved = resolver.skill(&target).unwrap();
    let skill = resolved
        .value()
        .expect("exact native generated key resolves");
    assert_eq!(skill.definition(), Some(&f.bindings.skill));
    assert_eq!(skill.target(), &target);
    // GeneratedSkillKey names the supplying parent and its skill-grant slot.
    // Entering the activation grant is a distinct provider traversal address.
    assert_eq!(skill.provider().key(), &f.provider(0));
    let mut traversed = f.provider(0);
    traversed.grant_path.push(f.bindings.grant.clone());
    let via_grant = resolver.provider(&traversed).unwrap();
    let entered = via_grant.value().expect("exact activation path resolves");
    assert_eq!(entered.key(), &traversed);
    for provider in [skill.provider(), entered] {
        let ProviderExposure::Skill { key, owner, .. } = provider.exposure() else {
            panic!("both addresses expose the same supplied skill");
        };
        assert_eq!(key, &f.generated(0));
        assert_eq!(owner, &SlotOwnerDefId::Skill(f.bindings.skill.clone()));
    }
    let mut wrong = f.generated(0);
    wrong.provider.root = ProviderRoot::EquipmentUse(f.build.equipment[0].id);
    assert!(
        resolver
            .skill(&SkillTarget::Generated(Box::new(wrong)))
            .unwrap()
            .value()
            .is_none()
    );

    // These are declared raw-input boundary tests, not source/effective-level
    // assertions about Firebolt at these levels.
    for n in [0, 1, 11, 20, 100] {
        let mut changed = f.clone();
        changed.build.items[0].modifiers[0].rolls[0].value = integer(n);
        changed.build.revision = changed.build.revision.checked_next().unwrap();
        let next = changed.plan().unwrap();
        let reused = next.evaluate(&mut scratch).unwrap();
        assert_eq!(reused, next.evaluate(&mut next.new_scratch()).unwrap());
        assert_eq!(
            value(&reused, &changed.raw_key(0)),
            Some(&EffectValue::Known { value: integer(n) })
        );
    }
    assert_eq!(report, plan.evaluate(&mut scratch).unwrap());

    let mut multi = f.clone();
    multi.extra_use(false, true); // Same physical item and modifier, distinct receiving use.
    multi.extra_use(true, true); // Same definition, different modifier instance and raw input.
    multi.extra_use(false, false);
    let multi_plan = multi.plan().unwrap();
    let multi_report = multi_plan.evaluate(&mut scratch).unwrap();
    for (index, n) in [(0, 11), (1, 11), (2, 20)] {
        assert_eq!(
            value(&multi_report, &multi.raw_key(index)),
            Some(&EffectValue::Known { value: integer(n) })
        );
    }
    assert_ne!(multi.generated(0), multi.generated(1));
    assert_ne!(multi.generated(1), multi.generated(2));
    assert!(value(&multi_report, &multi.raw_key(3)).is_none());
    assert_eq!(multi_report.effects.len(), 6);
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| scope.spawn(|| multi_plan.evaluate(&mut multi_plan.new_scratch()).unwrap()))
            .collect();
        for handle in handles {
            assert_eq!(handle.join().unwrap(), multi_report);
        }
    });

    for (input, code) in [
        (None, BindingIssueCode::RequiredValueMissing),
        (Some(integer(-1)), BindingIssueCode::OutOfRange),
        (Some(integer(101)), BindingIssueCode::OutOfRange),
        (
            Some(ParameterValue::Boolean(false)),
            BindingIssueCode::ValueKindMismatch,
        ),
    ] {
        let mut invalid = f.clone();
        invalid.build.items[0].modifiers[0].rolls = input
            .into_iter()
            .map(|value| ParameterAssignment {
                slot: invalid.bindings.level.clone(),
                value,
            })
            .collect();
        let schema = invalid.schema();
        let binding =
            bind_owned_request(schema.as_ref(), &invalid.request(), Default::default()).unwrap();
        assert_eq!(binding.schema(), SchemaBindingStatus::Invalid);
        assert!(binding.issues().iter().any(|issue| issue.code == code));
        assert!(
            matches!(invalid.plan(),Err(PlanError::Invalid(message)) if message == "owned request has invalid schema bindings")
        );
    }

    let mut partial = f.clone();
    for original in &partial.original_definitions {
        *partial
            .schema
            .definitions
            .iter_mut()
            .find(|row| row.address() == original.address())
            .unwrap() = original.clone();
    }
    partial.schema.slots = partial.original_slots.clone();
    *partial
        .rules
        .owners
        .iter_mut()
        .find(|row| row.owner == partial.original_owner.owner)
        .unwrap() = partial.original_owner.clone();
    let partial_plan = partial.plan().unwrap();
    assert!(
        partial_plan
            .gaps()
            .iter()
            .any(|gap| gap.reason == PlanGapReason::PartialPrograms
                && gap.subject.as_ref() == Some(&partial.original_owner.owner))
    );
    assert!(
        partial_plan
            .gaps()
            .iter()
            .any(|gap| gap.reason == PlanGapReason::PartialDeclarations)
    );
    assert!(
        !partial_plan
            .evaluate(&mut partial_plan.new_scratch())
            .unwrap()
            .gaps
            .is_empty()
    );
    // All runtime execution above used the original authored program unchanged.
    assert_eq!(
        f.rules.owners[0].programs.members,
        f.original_owner.programs.members
    );
}
