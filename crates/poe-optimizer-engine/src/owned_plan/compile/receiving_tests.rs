//! Cold receiving metadata binds occurrences, never numeric delivery authority.
use super::*;
use crate::owned_plan::compile::support_fixture as fixture;
use crate::owned_rules::RuleLimits;
use fixture::{Fixture, child_actor, child_grant, def, effect, key, occurrence, subject, target};
use poe_optimizer_core::owned_stages::*;
use poe_optimizer_data::{
    owned_rules::OwnedRulePackage, owned_schema::OwnedDefinitionSchemaPackage,
    owned_stages::OwnedEvaluationStages, owned_support_inputs::OwnedSupportInputBindings,
    owned_support_receiving::OwnedSupportReceiving, owned_supports::OwnedSupportPreparation,
};

fn output() -> DeclaredSlot<ActionOutputDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(def("ability")),
        slot: def("receiving-output"),
    }
}
fn activation(name: &str) -> DeclaredSlot<GrantSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Actor(def("family")),
        slot: def(name),
    }
}
fn support_owner() -> SchemaSubject {
    subject(def::<GemDefinition>("support"))
}
fn actor_endpoint(path: Vec<DeclaredSlot<GrantSlotDefId>>) -> SupportReceiverEndpoint {
    SupportReceiverEndpoint::Actor {
        path,
        admission: SupportAdmissionContext::AssignedSkill,
    }
}
fn action_endpoint(
    path: Vec<DeclaredSlot<GrantSlotDefId>>,
    admission: SupportAdmissionContext,
) -> SupportReceiverEndpoint {
    SupportReceiverEndpoint::Action {
        path,
        output: output(),
        selection: SupportActionSelection::AllDeclared,
        admission,
    }
}
fn source_fixture() -> Fixture {
    let mut f = fixture::generated_fixture();
    f.schema
        .definitions
        .push(DefinitionDescriptor::ActionPart(DefinitionEntry {
            id: def("part-two"),
            schema: SchemaState::Known(ActionPartSchema {}),
        }));
    for definition in &mut f.schema.definitions {
        if let DefinitionDescriptor::Skill(DefinitionEntry {
            id,
            schema: SchemaState::Known(schema),
        }) = definition
            && *id == def::<SkillDefinition>("ability")
        {
            schema.declarations.outputs.members.push(output());
        }
    }
    for slot in &mut f.schema.slots {
        if let SlotDescriptor::SkillGrant(DefinitionEntry {
            schema: SchemaState::Known(schema),
            ..
        }) = slot
            && schema.skill == def::<SkillDefinition>("ability")
        {
            schema.outputs.members.push(output());
        }
    }
    f.schema
        .slots
        .push(SlotDescriptor::ActionOutput(DefinitionEntry {
            id: output(),
            schema: SchemaState::Known(ActionOutputSchema {
                actor_role: DeclaredActorRole::ProviderActor,
                parts: DeclaredSet::complete(vec![def("part"), def("part-two")]),
                modes: DeclaredSet::complete(vec![def("mode")]),
                stat_sets: DeclaredSet::complete(vec![def("set")]),
                choices: DeclaredSet::complete(vec![]),
            }),
        }));
    // This ordinary Skill-owner action program must see receiver actions even
    // though no user query mentioned those actions before discovery.
    f.owner_mut(&subject(def::<SkillDefinition>("ability")))
        .programs
        .members
        .push(RuleProgram {
            id: key("action-observer"),
            context: RuleEntityKind::Action,
            reads: vec![],
            nodes: vec![fixture::bool_node("true", true)],
            effects: vec![effect(
                "observed",
                RuleEffectKind::Requirement {
                    satisfied: key("true"),
                    code: key("observed"),
                },
            )],
        });
    f
}
fn receiver_program(name: &str, context: RuleEntityKind, applicable: bool) -> RuleProgram {
    RuleProgram {
        id: key(name),
        context,
        reads: vec![],
        nodes: vec![fixture::bool_node("true", true)],
        effects: vec![effect(
            "result",
            if applicable {
                RuleEffectKind::SupportApplicability {
                    applicable: key("true"),
                }
            } else {
                RuleEffectKind::Requirement {
                    satisfied: key("true"),
                    code: key("delivery"),
                }
            },
        )],
    }
}
fn partial(owner: SchemaSubject) -> SchemaClosure {
    SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: owner,
            facet: SchemaFacet::GameRules,
            code: key("pending"),
        }],
    }
}
struct Harness {
    request: Arc<OwnedEvaluationRequest>,
    definitions: Arc<OwnedDefinitionSchemaPackage>,
    stored: OwnedRulePackage,
    rules: Arc<CompiledRulePackage>,
    receiving: OwnedSupportReceiving,
}
impl Harness {
    fn new(f: &Fixture, edit: impl FnOnce(&mut SupportReceivingInput)) -> Self {
        let args = fixture::compile_inputs(f, target(30, "first"));
        let mut rule_input = args.rules.input().clone();
        rule_input.operations_version = key(OWNED_RULE_OPERATIONS_V13);
        let row = rule_input
            .owners
            .iter_mut()
            .find(|r| r.owner == support_owner())
            .unwrap();
        row.programs.members.extend([
            receiver_program("actor-app", RuleEntityKind::Actor, true),
            receiver_program("actor-deliver", RuleEntityKind::Actor, false),
            receiver_program("action-app", RuleEntityKind::Action, true),
            receiver_program("action-deliver", RuleEntityKind::Action, false),
        ]);
        let stored =
            OwnedRulePackage::new(rule_input, args.definitions.as_ref(), Default::default())
                .unwrap();
        let rules = Arc::new(
            CompiledRulePackage::compile_stored(
                &stored,
                args.definitions.as_ref(),
                RuleLimits::default(),
            )
            .unwrap(),
        );
        let mut stage_input = args.stages.input().clone();
        stage_input.rules = *stored.identity();
        stage_input.stages.extend([
            EvaluationStage {
                id: key("applicable"),
                predecessors: vec![key("prepare")],
            },
            EvaluationStage {
                id: key("deliver"),
                predecessors: vec![key("applicable")],
            },
        ]);
        for (name, stage) in [
            ("actor-app", "applicable"),
            ("action-app", "applicable"),
            ("actor-deliver", "deliver"),
            ("action-deliver", "deliver"),
        ] {
            stage_input.programs.members.push(StagedRuleProgram {
                owner: support_owner(),
                program: key(name),
                stage: key(stage),
            });
        }
        let stages = OwnedEvaluationStages::new(
            stage_input,
            args.definitions.as_ref(),
            &stored,
            &args.routing,
            Default::default(),
        )
        .unwrap();
        let mut prep_input = args.preparation.input().clone();
        prep_input.rules = *stored.identity();
        let preparation = OwnedSupportPreparation::new(
            prep_input,
            args.definitions.as_ref(),
            &stored,
            Default::default(),
        )
        .unwrap();
        let mut input_input = args.inputs.input().clone();
        input_input.rules = *stored.identity();
        input_input.preparation = *preparation.identity();
        input_input.stages = *stages.identity();
        let inputs = OwnedSupportInputBindings::new(
            input_input,
            args.definitions.as_ref(),
            &stored,
            &preparation,
            &stages,
            Default::default(),
        )
        .unwrap();
        let mut input = SupportReceivingInput {
            schema_version: OWNED_SUPPORT_RECEIVING_VERSION,
            namespace: fixture::ns(),
            release: key("receiving"),
            definitions: args.definitions.identity().clone(),
            rules: *stored.identity(),
            preparation: *preparation.identity(),
            inputs: *inputs.identity(),
            stages: *stages.identity(),
            roles: vec![
                SupportReceivingRole {
                    id: key("actor"),
                    kind: SupportReceiverKind::Actor,
                },
                SupportReceivingRole {
                    id: key("action"),
                    kind: SupportReceiverKind::Action,
                },
            ],
            targets: vec![SupportTargetReceivingRoles {
                owner: SupportTargetDefinition::Skill(def("ability")),
                roles: DeclaredSet::complete(vec![
                    SupportReceivingRoleBinding {
                        role: key("actor"),
                        endpoints: DeclaredSet::complete(vec![actor_endpoint(vec![])]),
                    },
                    SupportReceivingRoleBinding {
                        role: key("action"),
                        endpoints: DeclaredSet::complete(vec![action_endpoint(
                            vec![],
                            SupportAdmissionContext::AssignedSkill,
                        )]),
                    },
                ]),
            }],
            supports: vec![SupportReceivingEntry {
                gem: def("support"),
                receivers: DeclaredSet::complete(vec![
                    SupportRolePrograms {
                        preparation: None,
                        role: key("actor"),
                        applicability: key("actor-app"),
                        delivery: vec![key("actor-deliver")],
                    },
                    SupportRolePrograms {
                        preparation: None,
                        role: key("action"),
                        applicability: key("action-app"),
                        delivery: vec![key("action-deliver")],
                    },
                ]),
            }],
        };
        edit(&mut input);
        let receiving = OwnedSupportReceiving::new(
            input,
            args.definitions.as_ref(),
            &stored,
            &preparation,
            &inputs,
            &stages,
            Default::default(),
        )
        .unwrap();
        Self {
            request: args.request,
            definitions: args.definitions,
            stored,
            rules,
            receiving,
        }
    }
    fn builder(&self) -> Builder<'_, OwnedDefinitionSchemaPackage> {
        let limits = PlanLimits::default();
        let mut builder = Builder::new(
            &self.request,
            self.definitions.as_ref(),
            &self.rules,
            OwnedOccurrenceResolver::new(self.definitions.as_ref(), &self.request, limits.binding)
                .unwrap(),
            RuleOperationsVersion::V13,
            limits,
        );
        builder.preparation = true;
        builder
    }
}

#[test]
fn generated_targets_preserve_entering_paths_siblings_and_actor_once_contexts() {
    let h = Harness::new(&source_fixture(), |_| {});
    let mut b = h.builder();
    let owners = b.discover().unwrap();
    assert!(b.actions.is_empty() && b.invocations.is_empty());
    let bound = b.receiving(&h.receiving).unwrap();
    assert_eq!(bound.assignments.len(), 6);
    assert_eq!(bound.package_identity, *h.receiving.identity());
    assert_eq!(
        b.actions.len(),
        6,
        "three exact skills with two declared parts each"
    );
    for (id, use_id, name) in [(60, 30, "first"), (62, 30, "second"), (64, 31, "first")] {
        let row = &bound.assignments[&occurrence(id)];
        assert!(row.complete);
        assert_eq!(row.availability, BoundReceivingAvailability::Bound);
        assert_eq!(row.target, target(use_id, name));
        assert_eq!(row.receivers.len(), 3);
        assert_eq!(
            row.receivers
                .iter()
                .filter(|r| matches!(r.context.receiver, SupportReceiverKey::Actor(_)))
                .count(),
            1
        );
        for receiver in &row.receivers {
            let expected_path = vec![child_grant(), activation(name)];
            assert_eq!(
                receiver.context.provider.root,
                ProviderRoot::SkillUse(occurrence(use_id))
            );
            assert_eq!(receiver.context.provider.grant_path, expected_path);
            match &receiver.context.receiver {
                SupportReceiverKey::Actor(actor) => {
                    assert_eq!(*actor, child_actor(use_id));
                    assert!(receiver.context.skill.is_none());
                }
                SupportReceiverKey::Action(action) => {
                    assert_eq!(action.action.actor, child_actor(use_id));
                    assert_eq!(receiver.context.skill, Some(target(use_id, name)));
                }
            }
            assert_eq!(
                receiver.context.admission,
                BoundSupportAdmission::AssignedSkill {
                    target: target(use_id, name)
                }
            );
        }
    }
    let a = &bound.assignments[&occurrence(60)].receivers[0].context;
    let duplicate_definition = &bound.assignments[&occurrence(61)].receivers[0].context;
    assert!(
        Arc::ptr_eq(a, duplicate_definition),
        "cold target/role contexts are shared across origins"
    );
    assert!(bound.classifies(occurrence(60), &support_owner(), &key("action-app")));
    assert!(!bound.classifies(
        occurrence(60),
        &fixture::summoner_owner(),
        &key("action-app")
    ));
    b.instantiate_discovered_owners(owners).unwrap();
    let observed: BTreeSet<_> = b
        .invocations
        .iter()
        .filter(|i| i.key.program == key("action-observer"))
        .map(|i| match &i.key.entity {
            ConcreteEntity::Action(action) => action.as_ref().clone(),
            _ => panic!("action context"),
        })
        .collect();
    assert_eq!(observed, b.actions);
}

fn authored_fixture() -> Fixture {
    let mut f = source_fixture();
    let mut sequences = BTreeMap::<SkillTarget, Vec<SupportOrigin>>::new();
    for (index, assignment) in f.build.supports.iter_mut().enumerate() {
        assignment.target = SkillTarget::Authored(occurrence(if index < 4 { 30 } else { 31 }));
        sequences
            .entry(assignment.target.clone())
            .or_default()
            .push(SupportOrigin::Assignment(assignment.id));
    }
    f.build.support_origins = Some(
        sequences
            .into_iter()
            .map(|(target, origins)| SupportOriginSequence { target, origins })
            .collect(),
    );
    f
}
fn authored_receivers(input: &mut SupportReceivingInput) {
    input.targets[0].owner = SupportTargetDefinition::Gem(def("summoner"));
    for row in &mut input.targets[0].roles.members {
        row.endpoints.members = if row.role == key("actor") {
            vec![actor_endpoint(vec![child_grant()])]
        } else {
            vec![action_endpoint(
                vec![child_grant(), activation("first")],
                SupportAdmissionContext::ReceivingSkill {
                    summoner_path: Some(vec![child_grant(), activation("second")]),
                },
            )]
        };
    }
}

#[test]
fn authored_gem_receivers_resolve_explicit_child_and_sibling_summoner_paths() {
    let h = Harness::new(&authored_fixture(), authored_receivers);
    let mut b = h.builder();
    b.discover().unwrap();
    let bound = b.receiving(&h.receiving).unwrap();
    assert_eq!(b.actions.len(), 4);
    for (id, use_id) in [(60, 30), (64, 31)] {
        let row = &bound.assignments[&occurrence(id)];
        assert!(row.complete);
        let receiver = row
            .receivers
            .iter()
            .find(|r| matches!(r.context.receiver, SupportReceiverKey::Action(_)))
            .unwrap();
        assert_eq!(receiver.context.skill, Some(target(use_id, "first")));
        assert_eq!(
            receiver.context.admission,
            BoundSupportAdmission::ReceivingSkill {
                target: target(use_id, "first"),
                summoner: Some(target(use_id, "second"))
            }
        );
    }
}

#[test]
fn missing_and_partial_local_inventories_do_not_classify_delivery_as_complete() {
    for mode in 0..4 {
        let h = Harness::new(&source_fixture(), |input| match mode {
            0 => input.supports.clear(),
            1 => input.targets.clear(),
            2 => input.supports[0].receivers.closure = partial(support_owner()),
            3 => {
                input.targets[0].roles.closure = partial(subject(def::<SkillDefinition>("ability")))
            }
            _ => unreachable!(),
        });
        let mut b = h.builder();
        b.discover().unwrap();
        let bound = b.receiving(&h.receiving).unwrap();
        assert!(bound.assignments.values().all(|row| !row.complete));
        assert!(!bound.classifies(occurrence(60), &support_owner(), &key("action-app")));
        assert!(
            b.gaps
                .iter()
                .any(|g| g.reason == PlanGapReason::PartialReceivers)
        );
    }
}

#[test]
fn complete_absent_roles_are_known_empty_without_inventing_receivers() {
    let h = Harness::new(&source_fixture(), |input| {
        input.targets[0].roles.members.clear()
    });
    let mut b = h.builder();
    b.discover().unwrap();
    let bound = b.receiving(&h.receiving).unwrap();
    assert!(
        bound
            .assignments
            .values()
            .all(|row| row.complete && row.receivers.is_empty())
    );
    assert!(bound.classifies(occurrence(60), &support_owner(), &key("action-app")));
    assert!(b.actions.is_empty());
    assert!(
        !b.gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialReceivers)
    );
}

#[test]
fn disabled_authored_target_is_unavailable_but_metadata_still_requires_coverage() {
    let mut f = authored_fixture();
    f.build
        .skills
        .iter_mut()
        .find(|s| s.id == occurrence(30))
        .unwrap()
        .enabled = false;
    let h = Harness::new(&f, authored_receivers);
    let mut b = h.builder();
    b.discover().unwrap();
    let bound = b.receiving(&h.receiving).unwrap();
    let disabled = &bound.assignments[&occurrence(60)];
    assert!(disabled.complete);
    assert_eq!(
        disabled.availability,
        BoundReceivingAvailability::Unavailable
    );
    assert!(disabled.receivers.is_empty());
    assert_eq!(
        bound.assignments[&occurrence(64)].availability,
        BoundReceivingAvailability::Bound
    );
    assert_eq!(b.actions.len(), 2);
    let missing = Harness::new(&f, |input| input.supports.clear());
    let mut b = missing.builder();
    b.discover().unwrap();
    let bound = b.receiving(&missing.receiving).unwrap();
    assert!(!bound.assignments[&occurrence(60)].complete);
}

#[test]
fn cross_role_aliases_cannot_multiply_one_actor_application() {
    let h = Harness::new(&source_fixture(), |input| {
        input.roles.push(SupportReceivingRole {
            id: key("actor-alias"),
            kind: SupportReceiverKind::Actor,
        });
        input.targets[0]
            .roles
            .members
            .push(SupportReceivingRoleBinding {
                role: key("actor-alias"),
                endpoints: DeclaredSet::complete(vec![actor_endpoint(vec![])]),
            });
        input.supports[0]
            .receivers
            .members
            .push(SupportRolePrograms {
                preparation: None,
                role: key("actor-alias"),
                applicability: key("actor-app"),
                delivery: vec![key("actor-deliver")],
            });
    });
    let mut b = h.builder();
    b.discover().unwrap();
    let error = b.receiving(&h.receiving).unwrap_err();
    assert!(
        matches!(error, PlanError::Invalid(message) if message.contains("competing applicability"))
    );
}

#[test]
fn raw_rule_compilation_cannot_claim_the_stored_receiving_package_identity() {
    let mut h = Harness::new(&source_fixture(), |_| {});
    h.rules = Arc::new(
        CompiledRulePackage::compile(
            h.stored.input(),
            h.definitions.as_ref(),
            RuleLimits::default(),
        )
        .unwrap(),
    );
    let mut b = h.builder();
    b.discover().unwrap();
    assert!(
        matches!(b.receiving(&h.receiving), Err(PlanError::Invalid(message)) if message.contains("bindings differ"))
    );
}

#[test]
fn cold_receiving_expansion_obeys_independent_inventory_and_shared_work_bounds() {
    let h = Harness::new(&source_fixture(), |_| {});
    for mode in 0..3 {
        let mut b = h.builder();
        b.discover().unwrap();
        match mode {
            0 => b.limits.max_owner_bindings = 5,
            1 => b.limits.max_providers = 1,
            2 => b.work = 1,
            _ => unreachable!(),
        }
        assert!(matches!(
            b.receiving(&h.receiving),
            Err(PlanError::Limit(_))
        ));
        if mode == 2 {
            assert_eq!(b.work, 0);
        }
    }
}

#[test]
fn disabled_generated_ancestor_keeps_unavailable_targets_without_invented_supply() {
    let mut f = source_fixture();
    f.build
        .skills
        .iter_mut()
        .find(|s| s.id == occurrence(30))
        .unwrap()
        .enabled = false;
    let h = Harness::new(&f, |_| {});
    let mut b = h.builder();
    b.discover().unwrap();
    let bound = b.receiving(&h.receiving).unwrap();
    for id in [60, 61, 62, 63] {
        let row = &bound.assignments[&occurrence(id)];
        assert!(row.complete);
        assert_eq!(row.availability, BoundReceivingAvailability::Unavailable);
        assert!(row.receivers.is_empty());
    }
    assert_eq!(
        bound.assignments[&occurrence(64)].availability,
        BoundReceivingAvailability::Bound
    );
    assert_eq!(b.actions.len(), 2);
}
