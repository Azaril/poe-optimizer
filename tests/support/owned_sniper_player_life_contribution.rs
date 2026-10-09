//! The shared Player emits actual intrinsic and prepared inherent Life in the
//! existing item/attribute graph. Only these two Actor programs are closed in
//! this finite test; equipment/rewards, final Life and the real Actor owner stay
//! outside that closure. Explicit RuleFacts below test only the bridge contract.
use super::*;
use poe_optimizer_core::owned_stages::{EvaluationStage, FrozenStageChannel, StageChannel};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, SchemaPackageInput};
use poe_optimizer_engine::owned_rules::{
    CompiledRulePackage, EffectDisposition, RuleFact, RuleScratch,
};
use poe_optimizer_import::owned_release::StagedOwnedRelease;

const INTRINSIC: &str = "intrinsic-player-life";
const BRIDGE: &str = "contribute-inherent-strength-life";
const STAGE: &str = "player-inherent-life-contribution";
const DISABLED: [&str; 3] = ["all-disabled", "strength-disabled", "life-disabled"];

#[derive(Clone)]
pub(super) struct Census {
    actual_owner: DefinitionRules,
    application: ExistingActorRuleApplication,
    original: Value,
}
fn owner() -> SchemaSubject {
    subject(d::<ActorDefinition>(0x332a))
}
fn inner(w: &mut World) -> &mut shared::World {
    &mut w.sniper.base.source.base.inner
}
pub(super) fn install(sniper: &mut sniper::World, endpoint: &StagedOwnedRelease) -> Census {
    minion_inherent_family::assert_component(endpoint);
    player_life_family::assert_component_with_reviewed_receivers(
        endpoint,
        &minion_inherent_family::dependencies().receiver_before,
        &minion_inherent_family::receivers(),
    );
    let recipe = &endpoint.input().recipe;
    let actual_owner = recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == owner())
        .unwrap()
        .clone();
    let mut expected = player_life_family::owner_before();
    expected
        .programs
        .members
        .push(player_life_family::program());
    assert_eq!(actual_owner.programs.closure, expected.programs.closure);
    for p in &expected.programs.members {
        assert_eq!(
            actual_owner.programs.members.iter().find(|a| a.id == p.id),
            Some(p)
        );
    }
    assert!(!actual_owner.programs.is_complete());
    let registry = recipe.rules.existing_actor_rules.as_ref().unwrap();
    assert!(registry.is_complete());
    assert_eq!(registry.members.len(), 1);
    let application = registry.members[0].clone();
    assert_eq!(application.owner, d(0x332a));
    assert_eq!(application.targets, vec![ExistingActorRuleTarget::Player]);
    let f = &mut sniper.base.source.base.inner;
    assert_eq!(
        f.build.character.level, 92,
        "actual imported Original05 level"
    );
    let descriptor = recipe
        .schema
        .definitions
        .iter()
        .find(|s| s.address() == d::<ActorDefinition>(0x332a).address())
        .unwrap()
        .clone();
    assert!(
        !f.schema
            .definitions
            .iter()
            .any(|s| s.address() == descriptor.address())
    );
    f.schema.definitions.push(descriptor);
    let programs: Vec<_> = actual_owner
        .programs
        .members
        .iter()
        .filter(|p| [INTRINSIC, BRIDGE].contains(&p.id.as_str()))
        .cloned()
        .collect();
    assert_eq!(programs.len(), 2);
    let finite = f.owner_mut(owner());
    assert!(finite.programs.members.is_empty());
    finite.programs = DeclaredSet::complete(programs);
    assert!(f.existing_actor_rules.is_none());
    f.existing_actor_rules = Some(registry.clone());
    let source = player_life_family::checked_source();
    let original = source["native_cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "original-05")
        .unwrap()
        .clone();
    assert_eq!(original["strength"], 27);
    assert_eq!(original["inherent_life"], 54);
    assert_eq!(original["emitted_count"], 1);
    Census {
        actual_owner,
        application,
        original,
    }
}
pub(super) fn configure(stages: &mut EvaluationStagesInput) {
    stages.stages.push(EvaluationStage {
        id: key(STAGE),
        predecessors: vec![key("inherent-strength-life")],
    });
    for row in &mut stages.programs.members {
        if row.owner == owner() && row.program == key(BRIDGE) {
            row.stage = key(STAGE);
        }
    }
    stages.frozen_channels.push(FrozenStageChannel {
        channel: StageChannel::Stat {
            scope: RuleEntityKind::Actor,
            stat: d(0x331a),
        },
        stage: key("inherent-strength-life"),
    });
    // The three flags are already frozen by inherent_life_native. No final Life
    // consumer exists here, so do not impose a new freeze on canonical311a.
}
fn player_rows(report: &SupportEffectsReport) -> Vec<&BoundEffectResult> {
    sniper::offering::effects(report)
        .effects
        .iter()
        .filter(|e| e.key.invocation.owner == owner())
        .collect()
}
pub(super) fn check(w: &World, report: &SupportEffectsReport, intrinsic: f64, inherent: f64) {
    let rows = player_rows(report);
    assert_eq!(rows.len(), 2, "one invocation per actual shared program");
    for (program, amount) in [(INTRINSIC, intrinsic), (BRIDGE, inherent)] {
        let matching: Vec<_> = rows
            .iter()
            .filter(|r| r.key.invocation.program == key(program))
            .collect();
        assert_eq!(matching.len(), 1);
        let row = matching[0];
        assert_eq!(
            row.key.invocation.entity,
            ConcreteEntity::Actor(ActorKey::Player)
        );
        assert_eq!(
            row.key.invocation.origin,
            RuleOrigin::ExistingActor {
                application: w.player_life.application.id.clone(),
                actor: ActorKey::Player,
            }
        );
        assert_eq!(
            row.target,
            BoundEffectTarget::Contribution {
                key: ContributionKey {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: d(0x311a),
                    kind: ContributionKind::Add,
                }
            }
        );
        assert_eq!(
            row.value,
            EffectValue::Known {
                value: quantity(amount, &d(0x3119))
            }
        );
    }
    let effects = sniper::offering::effects(report);
    for index in 0..2 {
        let rows: Vec<_> = effects
            .effects
            .iter()
            .filter(|e| {
                e.key.invocation.program == key("intrinsic-allied-minion-life")
                    && e.key.invocation.entity == ConcreteEntity::Actor(w.sniper.actor(index))
            })
            .collect();
        assert_eq!(
            rows.len(),
            1,
            "each exact minion retains its own intrinsic Life"
        );
        assert!(
            matches!(&rows[0].target, BoundEffectTarget::Contribution { key } if key.stat == d(0x311a)
            && key.kind == ContributionKind::Add && key.entity == ConcreteEntity::Actor(w.sniper.actor(index)))
        );
        assert!(matches!(&rows[0].value, EffectValue::Known { .. }));
        assert_ne!(rows[0].key.invocation.owner, owner());
    }
    assert!(
        !effects
            .values
            .iter()
            .any(|r| matches!(&r.key, PlanValueKey::Stat {stat,..} if *stat == d(0x311a))),
        "contributions are not a final Player or minion Life pool"
    );
}
fn unavailable(plan: &shared::Plan) {
    assert_eq!(
        plan.evaluate(&mut plan.new_scratch()).unwrap().outcome,
        SupportEffectsOutcome::Unavailable {
            cause: EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                read: None
            },
            input: None,
        }
    );
}

#[test]
#[ignore = "requires current Sniper release with the shared inherent-Life contribution"]
fn actual_player_level_and_calculated_strength_emit_two_exact_player_life_sources() {
    let w = World::load();
    check(
        &w,
        &w.evaluate(),
        1120.,
        w.player_life.original["inherent_life"].as_f64().unwrap(),
    );
    // Preserve the requested attack contexts required by this complete finite
    // graph. Clearing them exercises a separate, unsupported coverage contract.
    let reordered_queries = w
        .checked_plan_selecting(|_| {}, |requests| requests.reverse())
        .unwrap();
    let report = reordered_queries
        .evaluate(&mut reordered_queries.new_scratch())
        .unwrap();
    check(&w, &report, 1120., 54.);
}

#[test]
#[ignore = "requires current Sniper release with the shared inherent-Life contribution"]
fn missing_amount_or_application_never_fabricates_a_player_life_contribution() {
    let mut missing = World::load();
    missing
        .sniper
        .receivers
        .members
        .retain(|r| r.stat != d(0x331a));
    let report = missing.evaluate();
    let rows = player_rows(&report);
    let bridge = rows
        .iter()
        .find(|e| e.key.invocation.program == key(BRIDGE))
        .unwrap();
    assert_eq!(
        bridge.value,
        EffectValue::Unresolved {
            reason: PlanGapReason::MissingProducer,
            read: Some(key("amount"))
        }
    );
    let intrinsic = rows
        .iter()
        .find(|e| e.key.invocation.program == key(INTRINSIC))
        .unwrap();
    assert_eq!(
        intrinsic.value,
        EffectValue::Known {
            value: quantity(1120., &d(0x3119))
        }
    );
    // Absence of the applicability declaration means no shared invocation. There
    // is no final Life consumer in this component that could turn that into zero.
    for absent_registry in [false, true] {
        let mut missing = World::load();
        if absent_registry {
            inner(&mut missing).existing_actor_rules = None;
        } else {
            inner(&mut missing)
                .existing_actor_rules
                .as_mut()
                .unwrap()
                .members
                .clear();
        }
        assert!(player_rows(&missing.evaluate()).is_empty());
    }
}

#[test]
#[ignore = "requires current Sniper release with the shared inherent-Life contribution"]
fn actual_actor_partial_coverage_and_late_amount_stage_remain_refusals() {
    let mut partial = World::load();
    let actual = partial.player_life.actual_owner.clone();
    assert!(!actual.programs.is_complete());
    // Restore actual coverage on the explicitly retained two-program slice;
    // unrelated offhand/resistance bodies are not a second fixture dependency.
    inner(&mut partial).owner_mut(owner()).programs.closure = actual.programs.closure.clone();
    let plan = partial.plan();
    assert!(plan.gaps().iter().any(
        |g| g.subject.as_ref() == Some(&owner()) && g.reason == PlanGapReason::PartialPrograms
    ));
    unavailable(&plan);
    let mut partial_registry = World::load();
    inner(&mut partial_registry)
        .existing_actor_rules
        .as_mut()
        .unwrap()
        .closure = actual.programs.closure;
    let plan = partial_registry.plan();
    assert!(
        plan.gaps()
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialExistingActorRules)
    );
    unavailable(&plan);
    let w = World::load();
    let error = w
        .checked_plan_configured(|stages| {
            stages
                .programs
                .members
                .iter_mut()
                .find(|p| p.owner == owner() && p.program == key(BRIDGE))
                .unwrap()
                .stage = key("deliver");
        })
        .err()
        .expect("bridge cannot read the amount before its real producer");
    assert!(
        error.contains("stage") || error.contains("frozen"),
        "{error}"
    );
}

#[test]
#[ignore = "requires current Sniper release with the shared inherent-Life contribution"]
fn player_life_sources_reuse_scratch_and_keep_parallel_minions_independent() {
    let w = World::load();
    let p = w.plan();
    let mut scratch = p.new_scratch();
    let original = p.evaluate(&mut scratch).unwrap();
    check(&w, &original, 1120., 54.);
    let mut changed = w.clone();
    inner(&mut changed).build.character.level = 91;
    let q = changed.plan();
    let report = q.evaluate(&mut scratch).unwrap();
    check(&changed, &report, 1108., 54.);
    assert_eq!(p.evaluate(&mut scratch).unwrap(), original);
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..8).into_par_iter().for_each_init(
                || p.new_scratch(),
                |s, _| assert_eq!(p.evaluate(s).unwrap(), original),
            );
        });
}

// This is a compiled component test using explicit read facts, not a second
// build fixture, native source inventory, or injected values in the joined path.
fn component() -> (OwnedDefinitionSchemaPackage, CompiledRulePackage) {
    player_life_family::check_authored();
    let dependencies: Value = player_life_family::read("dependencies.json");
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: 6,
            namespace: d::<ActorDefinition>(0x332a).namespace().clone(),
            release: key("finite-inherent-life-bridge"),
            semantics_version: key("finite-inherent-life-bridge"),
            definitions: decode(&dependencies["definitions"]),
            slots: vec![],
        },
        Default::default(),
    )
    .unwrap();
    let input = RulePackageInput {
        support_discovery: None,
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: schema.input().namespace.clone(),
        release: key("finite-inherent-life-bridge"),
        semantics_version: schema.input().semantics_version.clone(),
        operations_version: key(OWNED_RULE_OPERATIONS_V22),
        definitions: schema.identity().clone(),
        tables: vec![],
        owners: vec![DefinitionRules {
            owner: owner(),
            programs: DeclaredSet::complete(vec![player_life_family::program()]),
        }],
        receivers: DeclaredSet::complete(vec![]),
        effect_applications: Some(DeclaredSet::complete(vec![])),
        contribution_queries: Some(DeclaredSet::complete(vec![])),
        existing_actor_rules: Some(decode(&dependencies["existing_actor_rules"])),
    };
    let rules = CompiledRulePackage::compile(&input, &schema, Default::default()).unwrap();
    (schema, rules)
}
fn facts(amount: Option<f64>, disabled: [Option<bool>; 3]) -> Vec<RuleFact> {
    amount
        .map(|a| RuleFact {
            read: key("amount"),
            value: quantity(a, &d(0x3119)),
        })
        .into_iter()
        .chain(
            DISABLED
                .into_iter()
                .zip(disabled)
                .filter_map(|(name, flag)| {
                    flag.map(|value| RuleFact {
                        read: key(name),
                        value: ParameterValue::Boolean(value),
                    })
                }),
        )
        .collect()
}
fn evaluate_component(
    schema: &OwnedDefinitionSchemaPackage,
    rules: &CompiledRulePackage,
    scratch: &mut RuleScratch,
    facts: &[RuleFact],
) -> EffectDisposition {
    let result = rules
        .evaluate(&owner(), &key(BRIDGE), facts, schema, scratch)
        .unwrap();
    assert_eq!(result.effects.len(), 1);
    assert_eq!(
        result.effects[0].effect,
        player_life_family::program().effects[0].effect
    );
    result.effects[0].disposition.clone()
}
#[test]
fn authored_bridge_preserves_all_retained_source_emissions_as_a_component() {
    let (schema, rules) = component();
    let mut scratch = rules.new_scratch();
    let source = player_life_family::checked_source();
    let rows = source["native_cases"].as_array().unwrap();
    assert_eq!(rows.len(), 13);
    for row in rows {
        let disabled = [
            "NoAttributeBonuses",
            "NoStrengthAttributeBonuses",
            "NoStrBonusToLife",
        ]
        .map(|f| Some(row["flags"][f].as_bool().unwrap()));
        let amount = row["inherent_life"].as_f64().unwrap();
        let expected = if row["emitted_count"] == 0 {
            EffectDisposition::Inactive
        } else {
            EffectDisposition::Applied {
                value: quantity(amount, &d(0x3119)),
            }
        };
        assert_eq!(
            evaluate_component(
                &schema,
                &rules,
                &mut scratch,
                &facts(Some(amount), disabled)
            ),
            expected,
            "{}",
            row["name"]
        );
    }
}
#[test]
fn bridge_component_distinguishes_enabled_zero_disabled_absence_and_missing_inputs() {
    let (schema, rules) = component();
    let mut scratch = rules.new_scratch();
    assert_eq!(
        evaluate_component(
            &schema,
            &rules,
            &mut scratch,
            &facts(Some(0.), [Some(false); 3])
        ),
        EffectDisposition::Applied {
            value: quantity(0., &d(0x3119))
        }
    );
    assert_eq!(
        evaluate_component(
            &schema,
            &rules,
            &mut scratch,
            &facts(None, [Some(false); 3])
        ),
        EffectDisposition::Unresolved {
            input: key("amount")
        }
    );
    for i in 0..3 {
        let mut flags = [Some(false); 3];
        flags[i] = Some(true);
        assert_eq!(
            evaluate_component(&schema, &rules, &mut scratch, &facts(None, flags)),
            EffectDisposition::Inactive
        );
        flags[i] = None;
        assert_eq!(
            evaluate_component(&schema, &rules, &mut scratch, &facts(Some(54.), flags)),
            EffectDisposition::Unresolved {
                input: key(DISABLED[i])
            }
        );
    }
}
