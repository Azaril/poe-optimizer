//! The authored reservation component, with explicitly supplied test inputs.
//! This reuses the existing source table/producer; it is not whole-build parity.
#[path = "support/minion_attack_source_fixture.rs"]
mod fixture;

use fixture::{def, key, quantity};
use poe_optimizer_core::{
    build_identity::{BuildInstanceId, BuildLineage, InstanceId},
    owned_build::*,
    owned_definitions::*,
    owned_project::*,
    owned_routing::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::{
    owned_routing::OwnedActionRouting, owned_schema::OwnedDefinitionSchemaPackage,
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::CompiledRulePackage};
use rayon::prelude::*;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::{Arc, OnceLock},
};

type Plan = OwnedEffectPlan<OwnedDefinitionSchemaPackage>;

fn asset(name: &str) -> Value {
    serde_json::from_slice(
        &fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../data/owned/poe2/3887ae68/sniper-reservation")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}

#[derive(Deserialize)]
struct Inputs {
    policy: UsagePolicyDefId,
    parameter: DeclaredSlot<ParameterSlotDefId>,
    gem: GemDefId,
    skill: SkillDefId,
    primary_supply: DeclaredSlot<SkillGrantSlotDefId>,
    entering_grant: DeclaredSlot<GrantSlotDefId>,
    parent_output: DeclaredSlot<ActionOutputDefId>,
    base_coefficient: StatDefId,
    base_program: OwnedDefinitionKey,
    base_table: OwnedDefinitionKey,
    usage_program: OwnedDefinitionKey,
    usage_effect: OwnedDefinitionKey,
    consumer_program: OwnedDefinitionKey,
    stats: BTreeMap<String, StatDefId>,
}
fn inputs() -> &'static Inputs {
    static INPUTS: OnceLock<Inputs> = OnceLock::new();
    INPUTS.get_or_init(|| serde_json::from_value(asset("native-inputs.json")).unwrap())
}
fn occurrence<T: BuildInstanceId>(index: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([61; 16]), index).unwrap())
}
fn provider(index: usize) -> ProviderKey {
    ProviderKey {
        root: ProviderRoot::SkillUse(occurrence(30 + index as u64)),
        grant_path: vec![],
    }
}
fn target(index: usize) -> SkillTarget {
    SkillTarget::Generated(Box::new(GeneratedSkillKey {
        provider: provider(index),
        slot: inputs().primary_supply.clone(),
    }))
}
fn action(index: usize) -> ActionSelection {
    ActionSelection {
        action: ActionKey {
            actor: ActorKey::Player,
            provider: ProviderKey {
                root: provider(index).root,
                grant_path: vec![inputs().entering_grant.clone()],
            },
            output: inputs().parent_output.clone(),
        },
        part: def(7),
        mode: def(8),
        stat_set: def(9),
    }
}
fn usage(index: usize, count: i64) -> UsagePolicySelection {
    UsagePolicySelection {
        policy: inputs().policy.clone(),
        target: UsageTarget::Skill(target(index)),
        parameters: vec![ParameterAssignment {
            slot: inputs().parameter.clone(),
            value: ParameterValue::Integer(BoundedInteger::new(count).unwrap()),
        }],
    }
}

// Explicit synthetic controls, never defaults attached to production data. The
// source replay below replaces every value with observed raw inputs/guard facts.
fn synthetic_inputs() -> BTreeMap<String, ParameterValue> {
    let mut values = BTreeMap::new();
    for (name, value, unit) in [
        ("extra_spirit", 0., 4),
        ("reservation_multiplier", 1., 1),
        ("reserved_increased", 0., 2),
        ("reserved_more", 1., 1),
        ("efficiency_increased", 0., 2),
        ("efficiency_more", 1., 1),
        ("free_count", 0., 0x295a),
    ] {
        values.insert(name.into(), quantity(value, unit));
    }
    for (name, value) in [
        ("has_reservation", true),
        ("multiple_reservation", true),
        ("reservation_becomes_cost", false),
        ("mana_cost_gain_as_reservation", false),
        ("flat_base_override_present", false),
        ("flat_forced_present", false),
        ("active_mine_count_present", false),
        ("spirit_to_life_conversion_active", false),
    ] {
        values.insert(name.into(), ParameterValue::Boolean(value));
    }
    values
}

struct World {
    inner: fixture::World,
    preferences: Vec<UsagePolicySelection>,
}
impl World {
    fn new(
        levels: [u16; 2],
        counts: [Option<i64>; 2],
        explicit: BTreeMap<String, ParameterValue>,
    ) -> Self {
        let mut inner = fixture::World::new(levels);
        let f = &mut inner.f;
        assert_eq!(f.build.gems[0].definition, inputs().gem);
        f.schema.schema_version = 5;
        let migration = asset("migration.json");
        for row in migration["schema"].as_array().unwrap() {
            match row["kind"].as_str().unwrap() {
                "definition" => f
                    .schema
                    .definitions
                    .push(serde_json::from_value(row["value"].clone()).unwrap()),
                "slot" => f
                    .schema
                    .slots
                    .push(serde_json::from_value(row["value"].clone()).unwrap()),
                other => panic!("unexpected schema row {other}"),
            }
        }
        let owners: Vec<DefinitionRules> =
            serde_json::from_value(migration["owners"].clone()).unwrap();
        for owner in owners {
            if let Some(existing) = f.owners.iter_mut().find(|row| row.owner == owner.owner) {
                // The actual existing base program/table is retained. Only this
                // finite test world has a complete owner and explicit producers.
                existing.programs.members.extend(owner.programs.members);
            } else {
                f.owners.push(owner);
            }
        }
        let parent = f.owner_mut(&SchemaSubject::Definition(inputs().skill.address()));
        assert!(
            parent
                .programs
                .members
                .iter()
                .any(|p| p.id == inputs().base_program)
        );
        parent.programs.members.push(RuleProgram {
            id: key("fixture-explicit-reservation-inputs"),
            context: RuleEntityKind::Action,
            reads: vec![],
            nodes: explicit
                .iter()
                .map(|(name, value)| RuleNode {
                    id: key(&name.replace('_', "-")),
                    expression: RuleExpression::Literal {
                        value: value.clone(),
                    },
                })
                .collect(),
            effects: explicit
                .keys()
                .map(|name| RuleEffect {
                    id: key(&name.replace('_', "-")),
                    when: None,
                    effect: RuleEffectKind::Derive {
                        entity: RuleEntity::Current,
                        stat: inputs().stats[name].clone(),
                        value: key(&name.replace('_', "-")),
                    },
                })
                .collect(),
        });
        Self {
            inner,
            preferences: counts
                .into_iter()
                .enumerate()
                .filter_map(|(i, count)| count.map(|count| usage(i, count)))
                .collect(),
        }
    }
    fn request(&self) -> OwnedEvaluationRequest {
        let b = &self.inner.f.build;
        let limits = OwnedInputLimits::default();
        let selection = VariantSelection {
            character: occurrence(50),
            equipment: occurrence(51),
            allocations: occurrence(52),
            skills: occurrence(53),
            choices: occurrence(54),
            active_weapon_loadout: b.active_weapon_loadout,
        };
        let project = BuildProject::new(
            ProjectInput {
                allocator: b.allocator,
                revision: b.revision,
                game_version: b.game_version.clone(),
                weapon_loadouts: b.weapon_loadouts.clone(),
                items: vec![],
                gems: b.gems.clone(),
                rewards: vec![],
                equipment: vec![],
                allocations: vec![],
                skills: b.skills.clone(),
                supports: vec![],
                payload_links: vec![],
                character_presets: vec![CharacterPreset {
                    id: selection.character,
                    class: b.character.class.clone(),
                    ascendancy: None,
                    level: b.character.level,
                    rewards: vec![],
                }],
                equipment_presets: vec![EquipmentPreset {
                    id: selection.equipment,
                    equipment: vec![],
                }],
                allocation_presets: vec![AllocationPreset {
                    id: selection.allocations,
                    allocations: vec![],
                    equipment: vec![],
                }],
                skill_presets: vec![SkillPreset {
                    intent: None,
                    id: selection.skills,
                    skills: b.skills.iter().map(|s| s.id).collect(),
                    supports: vec![],
                    support_origins: None,
                    payload_links: vec![],
                    usage_preferences: Some(self.preferences.clone()),
                }],
                choice_presets: vec![ChoicePreset {
                    id: selection.choices,
                    choices: vec![],
                    rewards: vec![],
                }],
                saved_variants: vec![],
            },
            limits,
        )
        .unwrap();
        compose_request(
            &project,
            &selection,
            None,
            ScenarioSpec::new(self.inner.f.scenario.clone(), limits).unwrap(),
            QuerySpec::new(self.inner.f.queries.clone(), limits).unwrap(),
            limits,
        )
        .unwrap()
    }
    fn compile(&self, operations: &str) -> Result<Plan> {
        let f = &self.inner.f;
        let schema = Arc::new(
            OwnedDefinitionSchemaPackage::new(f.schema.clone(), Default::default()).unwrap(),
        );
        let rules = Arc::new(
            CompiledRulePackage::compile(
                &RulePackageInput {
                    support_discovery: Some(SupportDiscoveryInput {
                        providers: f
                            .owners
                            .iter()
                            .map(|row| SupportSourceDomainDeclaration {
                                owner: row.owner.clone(),
                                domain: SchemaState::Known(
                                    SupportSourceDomain::AuthoredAssignmentsOnly,
                                ),
                            })
                            .collect(),
                    }),
                    existing_actor_rules: None,
                    contribution_queries: None,
                    schema_version: OWNED_RULE_PACKAGE_VERSION,
                    namespace: schema.input().namespace.clone(),
                    release: key("finite-sniper-reservation"),
                    semantics_version: key("finite-component-only"),
                    operations_version: key(operations),
                    definitions: schema.identity().clone(),
                    owners: f.owners.clone(),
                    tables: f.tables.clone(),
                    receivers: f.receivers.clone(),
                    effect_applications: Some(DeclaredSet::complete(vec![])),
                },
                schema.as_ref(),
                Default::default(),
            )
            .unwrap(),
        );
        let routing = Arc::new(
            OwnedActionRouting::new(
                ActionRoutingInput {
                    schema_version: OWNED_ACTION_ROUTING_VERSION,
                    namespace: schema.input().namespace.clone(),
                    release: key("finite-parent-action-context"),
                    definitions: schema.identity().clone(),
                    outputs: f.routes.clone(),
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
    fn plan(&self) -> Plan {
        self.compile(OWNED_RULE_OPERATIONS_V15).unwrap()
    }
    fn evaluate(&self) -> OwnedEffectsReport {
        let p = self.plan();
        p.evaluate(&mut p.new_scratch()).unwrap()
    }
}

fn value<'a>(report: &'a OwnedEffectsReport, index: usize, name: &str) -> &'a EffectValue {
    let stat = &inputs().stats[name];
    &report
        .values
        .iter()
        .find(|row| {
            row.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Action(Box::new(action(index))),
                    stat: stat.clone(),
                }
        })
        .expect("exact parent Action statistic")
        .value
}
fn amount(report: &OwnedEffectsReport, index: usize, name: &str) -> f64 {
    match value(report, index, name) {
        EffectValue::Known {
            value: ParameterValue::Quantity(value),
        } => value.value(),
        other => panic!("expected finite amount for {name}: {other:?}"),
    }
}
fn producer(report: &OwnedEffectsReport, index: usize) -> &BoundEffectResult {
    report
        .effects
        .iter()
        .find(|row| {
            row.key.invocation.owner == SchemaSubject::Definition(inputs().policy.address())
                && row.key.invocation.program == inputs().usage_program
                && row.key.effect == inputs().usage_effect
                && row.key.invocation.entity == ConcreteEntity::Skill(Box::new(target(index)))
        })
        .expect("exact occurrence usage producer")
}

#[test]
fn actual_base_and_authored_consumer_use_composed_counts_for_independent_occurrences() {
    let mut w = World::new([1, 40], [Some(1), Some(3)], synthetic_inputs());
    let r = w.evaluate();
    assert!(r.gaps.is_empty(), "{:?}", r.gaps);
    assert!(r.values.iter().any(|row| row.key
        == PlanValueKey::Stat {
            entity: ConcreteEntity::Action(Box::new(action(0))),
            stat: inputs().base_coefficient.clone(),
        }
        && row.value
            == EffectValue::Known {
                value: quantity(90., 4)
            }));
    assert_eq!(
        [amount(&r, 0, "per_use_flat"), amount(&r, 1, "per_use_flat")],
        [90., 26.]
    );
    assert_eq!(
        [amount(&r, 0, "counted_flat"), amount(&r, 1, "counted_flat")],
        [90., 78.]
    );
    for (index, count) in [(0, 1.), (1, 3.)] {
        let p = producer(&r, index);
        assert!(matches!(p.key.invocation.origin, RuleOrigin::Usage { .. }));
        assert_eq!(
            p.value,
            EffectValue::Known {
                value: quantity(count, 0x295a)
            }
        );
    }
    w.inner.f.scenario.usage = vec![usage(0, 4)];
    assert_eq!(
        w.request().scenario().input().usage,
        vec![usage(0, 4), usage(1, 3)]
    );
    let r = w.evaluate();
    assert_eq!(
        [amount(&r, 0, "counted_flat"), amount(&r, 1, "counted_flat")],
        [360., 78.]
    );
}

#[test]
fn source_floor_bias_rounding_order_and_fractional_free_count_are_preserved() {
    let mut data = synthetic_inputs();
    data.insert("extra_spirit".into(), quantity(9974., 4)); // explicit base10000 at level40
    data.insert(
        "reservation_multiplier".into(),
        quantity(1.2345999999999, 1),
    );
    let r = World::new([40, 40], [Some(1), Some(1)], data).evaluate();
    assert_eq!(amount(&r, 0, "counted_flat"), 12346.); // strict unbiased floor would produce12345
    let mut data = synthetic_inputs();
    data.insert("extra_spirit".into(), quantity(-25., 4)); // explicit base1 at level40
    data.insert(
        "reserved_more".into(),
        quantity(f64::from_bits(0.5_f64.to_bits() - 1), 1),
    );
    data.insert("free_count".into(), quantity(0.5, 0x295a));
    let r = World::new([40, 40], [Some(3), Some(0)], data).evaluate();
    assert_eq!(amount(&r, 0, "per_use_flat"), 1.); // source floor(x+0.5), not robust nearest
    assert_eq!(amount(&r, 0, "counted_flat"), 2.5);
    assert_eq!(amount(&r, 1, "counted_flat"), 0.);
}

#[test]
fn missing_inputs_and_unsupported_branches_never_gain_neutral_defaults() {
    for missing in [
        "extra_spirit",
        "reservation_multiplier",
        "reserved_increased",
        "reserved_more",
        "efficiency_increased",
        "efficiency_more",
        "free_count",
        "has_reservation",
        "multiple_reservation",
        "flat_base_override_present",
        "spirit_to_life_conversion_active",
    ] {
        let mut data = synthetic_inputs();
        data.remove(missing);
        let r = World::new([1, 40], [Some(1), Some(3)], data).evaluate();
        assert!(
            matches!(value(&r, 0, "counted_flat"), EffectValue::Unresolved { .. }),
            "{missing}"
        );
    }
    for (guard, unsupported) in [
        ("has_reservation", false),
        ("multiple_reservation", false),
        ("reservation_becomes_cost", true),
        ("mana_cost_gain_as_reservation", true),
        ("flat_base_override_present", true),
        ("flat_forced_present", true),
        ("active_mine_count_present", true),
        ("spirit_to_life_conversion_active", true),
    ] {
        let mut data = synthetic_inputs();
        data.insert(guard.into(), ParameterValue::Boolean(unsupported));
        let r = World::new([1, 40], [Some(1), Some(3)], data).evaluate();
        assert_eq!(
            value(&r, 0, "counted_flat"),
            &EffectValue::Inactive,
            "{guard}"
        );
        assert!(r.effects.iter().any(|row| row.key.invocation.program
            == inputs().consumer_program
            && row.key.effect.as_str() == "ordinary-flat-branch-required"
            && row.value
                == EffectValue::Known {
                    value: ParameterValue::Boolean(false)
                }));
    }
    let mut w = World::new([1, 40], [Some(1), Some(3)], synthetic_inputs());
    w.inner.missing_final_input();
    assert!(matches!(
        value(&w.evaluate(), 0, "counted_flat"),
        EffectValue::Unresolved { .. }
    ));
    let r = World::new([1, 40], [None, Some(3)], synthetic_inputs()).evaluate();
    assert!(matches!(
        value(&r, 0, "counted_flat"),
        EffectValue::Unresolved { .. }
    ));
    assert_eq!(amount(&r, 1, "counted_flat"), 78.);
}

#[test]
fn lazy_zero_does_not_demand_unused_divisors_and_active_zero_division_is_explicit() {
    let mut data = synthetic_inputs();
    data.insert("reserved_increased".into(), quantity(-100., 2));
    data.remove("efficiency_more");
    let r = World::new([1, 40], [Some(1), Some(3)], data).evaluate();
    assert_eq!(amount(&r, 0, "counted_flat"), 0.);
    let mut data = synthetic_inputs();
    data.insert("efficiency_increased".into(), quantity(-100., 2));
    let r = World::new([1, 40], [Some(1), Some(3)], data).evaluate();
    assert!(matches!(
        value(&r, 0, "counted_flat"),
        EffectValue::NumericalError { .. }
    ));
}

#[test]
fn publication_preserves_existing_partial_owner_and_missing_readiness() {
    let m = asset("migration.json");
    assert_eq!(
        m["contract"]["operations_version"],
        OWNED_RULE_OPERATIONS_V17
    );
    assert!(m.get("evaluation").is_none());
    assert!(m["tables"].as_array().unwrap().is_empty());
    assert!(m["query_targets"].as_array().unwrap().is_empty());
    assert!(m["owners"].as_array().unwrap().iter().all(|row| {
        row["programs"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["id"] != inputs().base_program.as_str())
    }));
    let mut w = World::new([1, 40], [Some(1), Some(3)], synthetic_inputs());
    assert!(
        w.inner
            .f
            .tables
            .iter()
            .any(|table| table.id == inputs().base_table)
    );
    let owner: DefinitionRules = serde_json::from_value(m["owners"][1].clone()).unwrap();
    assert!(!owner.programs.is_complete());
    w.inner.f.owner_mut(&owner.owner).programs.closure = owner.programs.closure;
    assert!(
        matches!(w.compile(OWNED_RULE_OPERATIONS_V17),Err(PlanError::Invalid(message))
        if message=="operation v16 requires checked readiness stages")
    );
    let r = w.evaluate();
    assert!(!r.gaps.is_empty());
    assert!(!matches!(
        value(&r, 0, "counted_flat"),
        EffectValue::Known { .. }
    ));
}

#[test]
fn disabled_occurrence_and_reused_scratch_cannot_retain_prior_reservation() {
    let a = World::new([1, 40], [Some(1), Some(3)], synthetic_inputs()).plan();
    let b = World::new([40, 1], [Some(4), Some(0)], synthetic_inputs()).plan();
    let expected = [
        a.evaluate(&mut a.new_scratch()).unwrap(),
        b.evaluate(&mut b.new_scratch()).unwrap(),
    ];
    let mut scratch = a.new_scratch();
    for (plan, want) in [(&a, &expected[0]), (&b, &expected[1]), (&a, &expected[0])] {
        assert!(
            plan.evaluate(&mut scratch).unwrap() == *want,
            "A/B/A complete report differs"
        );
    }
    let mut missing = World::new([1, 40], [Some(1), Some(3)], synthetic_inputs());
    missing.inner.missing_final_input();
    let missing = missing.plan();
    assert!(matches!(
        value(&missing.evaluate(&mut scratch).unwrap(), 0, "counted_flat"),
        EffectValue::Unresolved { .. }
    ));
    assert!(a.evaluate(&mut scratch).unwrap() == expected[0]);
    let mut disabled = World::new([1, 40], [Some(1), Some(3)], synthetic_inputs());
    disabled.inner.f.build.skills[0].enabled = false;
    let disabled = disabled.plan();
    let r = disabled.evaluate(&mut scratch).unwrap();
    assert!(!r.effects.iter().any(|row| row.key.invocation.entity
        == ConcreteEntity::Action(Box::new(action(0)))
        && matches!(row.value, EffectValue::Known { .. })));
    // The finite fixture deliberately retains its disabled parent/child query
    // selections. Their unresolved census conservatively prevents complete
    // contributor coverage; that must not become a stale known reservation.
    assert!(matches!(
        value(&r, 1, "counted_flat"),
        EffectValue::Unresolved {
            reason: PlanGapReason::IncompleteContributors,
            ..
        }
    ));
    assert!(a.evaluate(&mut scratch).unwrap() == expected[0]);
}

#[test]
fn rayon_workers_privately_reuse_plans_and_preserve_complete_reports() {
    let plans = Arc::new([
        World::new([1, 40], [Some(1), Some(3)], synthetic_inputs()).plan(),
        World::new([40, 1], [Some(4), Some(0)], synthetic_inputs()).plan(),
    ]);
    let expected = [
        plans[0].evaluate(&mut plans[0].new_scratch()).unwrap(),
        plans[1].evaluate(&mut plans[1].new_scratch()).unwrap(),
    ];
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let reports: Vec<_> = pool.install(|| {
        (0..8)
            .into_par_iter()
            .map_init(
                || plans[0].new_scratch(),
                |scratch, batch| {
                    (0..9)
                        .map(|step| {
                            let index = (batch + step) % 2;
                            (index, plans[index].evaluate(scratch).unwrap())
                        })
                        .collect::<Vec<_>>()
                },
            )
            .collect()
    });
    for batch in reports {
        for (index, report) in batch {
            assert!(report == expected[index]);
        }
    }
}

fn observed_number(value: &Value, name: &str) -> f64 {
    value[name]
        .as_f64()
        .unwrap_or_else(|| panic!("missing observed number {name}"))
}
fn observed_flag(value: &Value, name: &str) -> bool {
    value[name]
        .as_bool()
        .unwrap_or_else(|| panic!("missing observed flag {name}"))
}
fn observed_inputs(row: &Value) -> BTreeMap<String, ParameterValue> {
    let raw = &row["inputs"];
    let reservation = &row["reservation"];
    let provenance = &row["level_provenance"];
    let mut data = BTreeMap::new();
    for (target, source, unit) in [
        ("extra_spirit", "extra_spirit", 4),
        ("reservation_multiplier", "reservation_multiplier", 1),
        ("reserved_increased", "reserved_inc", 2),
        ("reserved_more", "reserved_more", 1),
        ("efficiency_increased", "efficiency_inc", 2),
        ("efficiency_more", "efficiency_more", 1),
    ] {
        data.insert(target.into(), quantity(observed_number(raw, source), unit));
    }
    data.insert(
        "free_count".into(),
        quantity(observed_number(reservation, "free_count"), 0x295a),
    );
    for (target, source) in [
        ("has_reservation", "has_reservation"),
        ("multiple_reservation", "multiple"),
        ("reservation_becomes_cost", "becomes_cost"),
        ("mana_cost_gain_as_reservation", "mana_as_reservation"),
        ("flat_forced_present", "forced_flat_present"),
        ("active_mine_count_present", "mine_count_present"),
    ] {
        data.insert(
            target.into(),
            ParameterValue::Boolean(observed_flag(reservation, source)),
        );
    }
    data.insert(
        "flat_base_override_present".into(),
        ParameterValue::Boolean(
            observed_flag(raw, "skill_data_flat_present")
                || observed_flag(provenance, "no_reservation_active")
                || observed_flag(provenance, "selected_stat_set_flat_override_present"),
        ),
    );
    data.insert(
        "spirit_to_life_conversion_active".into(),
        ParameterValue::Boolean(observed_number(reservation, "spirit_to_life") > 0.),
    );
    data
}

#[test]
#[ignore = "requires the authenticated optional full-source reports named by authoring.json"]
fn authenticated_source_inputs_replay_the_authored_component_in_every_observed_stage() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let authoring = asset("authoring.json");
    let proof = &authoring["source_validation"];
    assert_eq!(proof["status"], "passed");
    let mut reports = vec![];
    for (path, size, hash) in [
        ("evidence_json", "evidence_bytes", "evidence_sha256"),
        (
            "evidence_on_json",
            "evidence_on_bytes",
            "evidence_on_sha256",
        ),
    ] {
        let bytes = fs::read(root.join(proof[path].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len() as u64, proof[size].as_u64().unwrap());
        assert_eq!(format!("{:x}", Sha256::digest(&bytes)), proof[hash]);
        reports.push(bytes);
    }
    assert!(reports[0] == reports[1], "complete JIT reports differ");
    let report: Value = serde_json::from_slice(&reports[0]).unwrap();
    assert_eq!(
        report["manifest_sha256"],
        authoring["source_manifest_sha256"]
    );
    assert_eq!(report["source_revision"], authoring["source_revision"]);
    assert_eq!(report["native_build_parity"], false);
    assert_eq!(report["native_inventory_authority"], false);
    let usage_row = asset("usage.json");
    let mut sniper_rows = 0;
    let mut free_count_rows = 0;
    let mut ambiguous_rows = 0;
    let mut excluded_other_profile_rows = 0;
    let mut zero_rows = 0;
    let mut stages = std::collections::BTreeSet::new();
    for case in report["cases"].as_array().unwrap() {
        for (stage, state) in case["states"].as_object().unwrap() {
            stages.insert(stage.clone());
            for saved in state["saved"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["selected"] == true)
            {
                for mode in ["MAIN", "CALCS"] {
                    for row in saved[mode].as_array().unwrap() {
                        let sniper = row["effect"] == usage_row["skill_id"];
                        // Other profiles are admitted only as arithmetic controls
                        // for the genuine free-count branch, never as Sniper data.
                        if !sniper {
                            let free_control = matches!(
                                case["name"].as_str(),
                                Some(
                                    "warrior-free-count-1"
                                        | "warrior-free-count-2"
                                        | "warrior-free-count-3"
                                )
                            );
                            let has_free_count =
                                observed_number(&row["reservation"], "free_count") > 0.;
                            if !free_control || !has_free_count {
                                if has_free_count {
                                    excluded_other_profile_rows += 1;
                                }
                                continue;
                            }
                        }
                        if saved["group_attributes"].get("groupCount").is_none()
                            && row["count_candidates"].as_array().unwrap().len() != 1
                        {
                            ambiguous_rows += 1;
                            continue;
                        }
                        let selected = saved["group_attributes"]
                            .get("groupCount")
                            .unwrap_or(&saved["attributes"]["count"])
                            .as_str()
                            .unwrap()
                            .parse::<i64>()
                            .unwrap();
                        assert!((0..=4).contains(&selected));
                        assert_eq!(observed_number(row, "count"), selected as f64);
                        assert_eq!(row["physical_source_exact"], true);
                        assert_eq!(row["group_exact"], true);
                        let raw = &row["inputs"];
                        let provenance = &row["level_provenance"];
                        assert!(observed_flag(raw, "final_level_flat_present"));
                        assert!(!observed_flag(raw, "skill_data_flat_present"));
                        assert!(!observed_flag(provenance, "no_reservation_active"));
                        assert!(!observed_flag(
                            provenance,
                            "selected_stat_set_flat_override_present"
                        ));
                        let coefficient = observed_number(raw, "final_level_flat");
                        assert_eq!(
                            coefficient,
                            observed_number(
                                &provenance["source_level_row"],
                                "spiritReservationFlat"
                            )
                        );
                        let final_level = observed_number(row, "final_level");
                        assert_eq!(final_level.fract(), 0.);
                        assert!((1. ..=40.).contains(&final_level));
                        let mut world = World::new(
                            [final_level as u16; 2],
                            [Some(selected), Some(0)],
                            observed_inputs(row),
                        );
                        let finalizer = world
                            .inner
                            .f
                            .owner_mut(&SchemaSubject::Definition(inputs().gem.address()))
                            .programs
                            .members
                            .iter_mut()
                            .find(|p| p.id.as_str() == "fixture-explicit-final-level")
                            .unwrap();
                        finalizer
                            .nodes
                            .iter_mut()
                            .find(|n| n.id.as_str() == "quality")
                            .unwrap()
                            .expression = RuleExpression::Literal {
                            value: quantity(observed_number(row, "final_quality"), 2),
                        };
                        if !sniper {
                            // Explicit observed coefficient only in this fixture.
                            // Do not pretend the Sniper table describes Warriors.
                            let program = world
                                .inner
                                .f
                                .owner_mut(&SchemaSubject::Definition(inputs().skill.address()))
                                .programs
                                .members
                                .iter_mut()
                                .find(|p| p.id == inputs().base_program)
                                .unwrap();
                            program.id = key("fixture-observed-other-profile-coefficient");
                            program
                                .nodes
                                .iter_mut()
                                .find(|n| n.id.as_str() == "reservation")
                                .unwrap()
                                .expression = RuleExpression::Literal {
                                value: quantity(coefficient, 4),
                            };
                            free_count_rows += 1;
                        } else {
                            sniper_rows += 1;
                        }
                        let native = world.evaluate();
                        assert!(
                            native.gaps.is_empty(),
                            "source component has unexpected gaps"
                        );
                        let base = native
                            .values
                            .iter()
                            .find(|v| {
                                v.key
                                    == PlanValueKey::Stat {
                                        entity: ConcreteEntity::Action(Box::new(action(0))),
                                        stat: inputs().base_coefficient.clone(),
                                    }
                            })
                            .unwrap();
                        assert_eq!(
                            base.value,
                            EffectValue::Known {
                                value: quantity(coefficient, 4)
                            }
                        );
                        let before = &row["captured"]["pools"]["Spirit"]["before_count"]["values"];
                        assert_eq!(observed_number(before, "reservedPercent"), 0.);
                        assert_eq!(
                            amount(&native, 0, "per_use_flat"),
                            observed_number(before, "reservedFlat"),
                            "{} / {stage} / {mode} / source{}",
                            case["name"],
                            saved["source_ordinal"]
                        );
                        if observed_flag(&row["reservation"], "spirit_result_present") {
                            assert_eq!(
                                amount(&native, 0, "counted_flat"),
                                observed_number(&row["reservation"], "spirit_result")
                            );
                        } else {
                            // Native arithmetic zero is not an invented source
                            // field. Source omission is checked independently.
                            assert!(row["reservation"].get("spirit_result").is_none());
                            assert_eq!(amount(&native, 0, "counted_flat"), 0.);
                            zero_rows += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(
        stages,
        ["fresh", "rebuilt_once", "rebuilt_twice"]
            .map(str::to_owned)
            .into_iter()
            .collect()
    );
    assert!(
        sniper_rows >= 100,
        "all nonambiguous source stages should replay"
    );
    assert!(
        free_count_rows >= 18,
        "three free-count controls in both modes and all stages"
    );
    assert!(
        zero_rows >= 12,
        "zero count and source free-count omission controls"
    );
    assert!(
        ambiguous_rows >= 12,
        "ambiguous source fallback must stay outside consumer correspondence"
    );
    // Original01's other-profile observations are not these explicit controls;
    // one has raw count='nil' and uses a source fallback not admitted by UsageV2.
    assert_eq!(excluded_other_profile_rows, 12);
}
