//! Cross-entity reads keep the selected occurrence's declared channel scope.
use super::*;
use poe_optimizer_core::owned_readiness::*;

fn actor_owner() -> SchemaSubject {
    SchemaSubject::Definition(id::<ActorDefinition>("shared-player").address())
}

fn fixture(read: PlayerEquipmentSlotRead, value_type: ComputedValueType) -> Fixture {
    let mut f = Fixture::new();
    let mut schema = f.schema.input().clone();
    schema.definitions.extend([
        DefinitionDescriptor::Actor(entry(
            id("shared-player"),
            ActorSchema {
                declarations: DeclaredSlots {
                    parameters: empty(),
                    choices: empty(),
                    grants: empty(),
                    actors: empty(),
                    skill_grants: empty(),
                    outputs: empty(),
                    sockets: empty(),
                },
            },
        )),
        DefinitionDescriptor::Stat(entry(
            id("equipment-amount"),
            StatSchema {
                value: ComputedValueType::Integer,
                targets: vec![RuleEntityKind::EquipmentUse],
            },
        )),
    ]);
    f.schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
    let identity = f.schema.identity().clone();
    f.change_rules(|rules| {
        rules.definitions = identity.clone();
        rules.operations_version = key(OWNED_RULE_OPERATIONS_V21);
        rules.effect_applications = Some(empty());
        rules.contribution_queries = Some(empty());
        rules.existing_actor_rules =
            Some(DeclaredSet::complete(vec![ExistingActorRuleApplication {
                id: key("player"),
                owner: id("shared-player"),
                targets: vec![ExistingActorRuleTarget::Player],
            }]));
        let consumer = rules
            .owners
            .iter_mut()
            .find(|v| v.owner == owner("b"))
            .unwrap();
        consumer.owner = actor_owner();
        consumer.programs.members[0].reads = vec![RuleRead {
            id: key("slot"),
            value_type,
            source: RuleReadSource::PlayerEquipmentSlot {
                slot: id("weapon"),
                read,
            },
        }];
    });
    f.change_routing(|routing| routing.definitions = identity.clone());
    f.input.definitions = identity;
    f.input.schema_version = OWNED_EVALUATION_STAGES_V4;
    f.input.effect_applications = Some(empty());
    for row in &mut f.input.programs.members {
        if row.owner == owner("b") {
            row.owner = actor_owner();
        }
    }
    f.input.readiness = Some(ReadinessInput {
        skills: vec![],
        programs: DeclaredSet::complete(
            f.input
                .programs
                .members
                .iter()
                .map(|row| ReadinessProgram {
                    owner: row.owner.clone(),
                    program: row.program.clone(),
                    phase: ReadinessPhase::Execution,
                    role: ReadinessProgramRole::Execution,
                    outputs: vec![],
                })
                .collect(),
        ),
    });
    f
}

#[test]
fn selected_equipment_stat_and_capability_reads_obey_frozen_channels() {
    for (read, value_type, channel) in [
        (
            PlayerEquipmentSlotRead::Stat {
                stat: id("equipment-amount"),
            },
            ComputedValueType::Integer,
            StageChannel::Stat {
                scope: RuleEntityKind::EquipmentUse,
                stat: id("equipment-amount"),
            },
        ),
        (
            PlayerEquipmentSlotRead::Capability {
                capability: id("eligibility"),
            },
            ComputedValueType::Boolean,
            StageChannel::Capability {
                scope: RuleEntityKind::EquipmentUse,
                capability: id("eligibility"),
            },
        ),
    ] {
        let mut f = fixture(read, value_type);
        f.input.frozen_channels.push(FrozenStageChannel {
            channel,
            stage: key("prepare"),
        });
        f.package().unwrap();
        f.input.frozen_channels.last_mut().unwrap().stage = key("finish");
        assert!(matches!(
            f.package(),
            Err(StageStorageError::Invalid(
                "frozen channel read occurs before or outside frozen stage"
            ))
        ));
    }
}

#[test]
fn occupancy_is_structural_and_does_not_demand_an_unread_computed_channel() {
    let mut f = fixture(
        PlayerEquipmentSlotRead::Occupied,
        ComputedValueType::Boolean,
    );
    f.input.frozen_channels.push(FrozenStageChannel {
        channel: StageChannel::Stat {
            scope: RuleEntityKind::EquipmentUse,
            stat: id("equipment-amount"),
        },
        stage: key("finish"),
    });
    f.package().unwrap();
}
