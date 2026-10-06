//! Actual Ice Nova action correspondence and structural supply only.
//! The finite fixture excludes unconverted mechanics; it never supplies damage,
//! final skill inputs, infusion state, or a source-selected MAIN/CALCS context.
#[allow(dead_code)]
#[path = "support/owned_plan_fixture.rs"]
mod base;

use base::{key, occurrence};
use poe_optimizer_core::{
    owned_binding::*, owned_build::*, owned_definitions::*, owned_routing::*, owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::{
    owned_routing::OwnedActionRouting, owned_schema::OwnedDefinitionSchemaPackage,
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::CompiledRulePackage};
use rayon::prelude::*;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::sync::Arc;
use std::{collections::BTreeSet, fs, path::PathBuf, sync::OnceLock};

type Plan = OwnedEffectPlan<OwnedDefinitionSchemaPackage>;

fn asset(name: &str) -> Value {
    serde_json::from_slice(
        &fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../data/owned/poe2/3887ae68/ice-nova-actions")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}

#[derive(Deserialize)]
struct StatSet {
    source_index: u32,
    stat_set: ActionStatSetDefId,
}
#[derive(Deserialize)]
struct Inputs {
    schema_version: u32,
    physical_gem: GemDefId,
    primary_skill: SkillDefId,
    primary_supply: DeclaredSlot<SkillGrantSlotDefId>,
    entering_grant: DeclaredSlot<GrantSlotDefId>,
    output: DeclaredSlot<ActionOutputDefId>,
    part: ActionPartDefId,
    mode: ActionModeDefId,
    stat_sets: Vec<StatSet>,
}
fn inputs() -> &'static Inputs {
    static INPUTS: OnceLock<Inputs> = OnceLock::new();
    INPUTS.get_or_init(|| serde_json::from_value(asset("correspondence.json")).unwrap())
}

// Namespace only the generic character/encounter skeleton. The packet and its
// physical dependencies are copied without changing their owned identities.
fn namespace<T: Serialize + DeserializeOwned>(source: &T) -> T {
    fn walk(value: &mut Value) {
        match value {
            Value::Object(fields) if fields.get("game") == Some(&json!("owned-plan-test")) => {
                *value = serde_json::to_value(inputs().physical_gem.namespace()).unwrap();
            }
            Value::Object(fields) => fields.values_mut().for_each(walk),
            Value::Array(values) => values.iter_mut().for_each(walk),
            _ => {}
        }
    }
    let mut value = serde_json::to_value(source).unwrap();
    walk(&mut value);
    serde_json::from_value(value).unwrap()
}

fn finite_ports(ports: &mut DeclaredSlots) {
    ports.parameters.closure = SchemaClosure::Complete;
    ports.choices.closure = SchemaClosure::Complete;
    ports.grants.closure = SchemaClosure::Complete;
    ports.actors.closure = SchemaClosure::Complete;
    ports.skill_grants.closure = SchemaClosure::Complete;
    ports.outputs.closure = SchemaClosure::Complete;
    ports.sockets.closure = SchemaClosure::Complete;
}

fn provider(copy: usize) -> ProviderKey {
    ProviderKey {
        root: ProviderRoot::SkillUse(occurrence(30 + copy as u64)),
        grant_path: vec![],
    }
}
fn action(copy: usize, set: usize) -> ActionSelection {
    let input = inputs();
    ActionSelection {
        action: ActionKey {
            actor: ActorKey::Player,
            provider: ProviderKey {
                root: provider(copy).root,
                grant_path: vec![input.entering_grant.clone()],
            },
            output: input.output.clone(),
        },
        part: input.part.clone(),
        mode: input.mode.clone(),
        stat_set: input.stat_sets[set].stat_set.clone(),
    }
}

struct World {
    f: base::Fixture,
}
impl World {
    fn new(finite_topology_only: bool) -> Self {
        let input = inputs();
        let migration = asset("migration.json");
        let dependencies: Value =
            serde_json::from_str(include_str!("support/ice_nova_physical_dependencies.json"))
                .unwrap();
        assert_eq!(dependencies["source"]["input"], migration["before"]);
        let mut f = base::Fixture::new();
        f.schema = namespace(&f.schema);
        f.build = namespace(&f.build);
        f.scenario = namespace(&f.scenario);
        f.queries = namespace(&f.queries);
        f.schema.schema_version = 5;
        f.build.items.clear();
        f.build.equipment.clear();
        f.owners.clear();
        f.routes.clear();
        f.tables.clear();
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
        f.schema.definitions.extend(
            serde_json::from_value::<Vec<DefinitionDescriptor>>(
                dependencies["definitions"].clone(),
            )
            .unwrap(),
        );
        let parameters: Vec<SlotDescriptor> =
            serde_json::from_value(dependencies["slots"].clone()).unwrap();
        f.schema.slots.extend(parameters.clone());
        f.owners = serde_json::from_value(migration["owners"].clone()).unwrap();
        assert!(migration["tables"].as_array().unwrap().is_empty());
        assert!(migration["receivers"].as_array().unwrap().is_empty());

        if finite_topology_only {
            // The finite test contains only the declared supply and alternative
            // addresses. It adds no numerical producer, and never mutates the
            // actual packet. The separate Partial test retains every closure.
            for row in &mut f.schema.definitions {
                match row {
                    DefinitionDescriptor::Gem(e) if e.id == input.physical_gem => {
                        let SchemaState::Known(gem) = &mut e.schema else {
                            panic!("known gem")
                        };
                        gem.skills.closure = SchemaClosure::Complete;
                        gem.quality.allowed_kinds.closure = SchemaClosure::Complete;
                        finite_ports(&mut gem.declarations);
                    }
                    DefinitionDescriptor::Skill(e) if e.id == input.primary_skill => {
                        let SchemaState::Known(skill) = &mut e.schema else {
                            panic!("known skill")
                        };
                        finite_ports(&mut skill.declarations);
                    }
                    _ => {}
                }
            }
            for row in &mut f.schema.slots {
                match row {
                    SlotDescriptor::SkillGrant(e) if e.id == input.primary_supply => {
                        let SchemaState::Known(supply) = &mut e.schema else {
                            panic!("known supply")
                        };
                        supply.outputs.closure = SchemaClosure::Complete;
                    }
                    SlotDescriptor::ActionOutput(e) if e.id == input.output => {
                        let SchemaState::Known(output) = &mut e.schema else {
                            panic!("known output")
                        };
                        output.choices.closure = SchemaClosure::Complete;
                    }
                    _ => {}
                }
            }
            for owner in &mut f.owners {
                owner.programs.closure = SchemaClosure::Complete;
            }
        }
        for owner in [
            SchemaSubject::Definition(f.build.character.class.address()),
            SchemaSubject::Definition(f.scenario.enemy.encounter.address()),
            SchemaSubject::Slot(GrantSlotDefId::address(&input.entering_grant)),
        ] {
            f.owners.push(DefinitionRules {
                owner,
                programs: DeclaredSet::complete(vec![]),
            });
        }
        if finite_topology_only {
            for owner in [
                SchemaSubject::Definition(input.primary_skill.address()),
                SchemaSubject::Slot(ActionOutputDefId::address(&input.output)),
            ] {
                f.owners.push(DefinitionRules {
                    owner,
                    programs: DeclaredSet::complete(vec![]),
                });
            }
            f.routes.push(ActionOutputRoutes {
                output: input.output.clone(),
                routes: DeclaredSet::complete(vec![]),
                source_selectors: Some(DeclaredSet::complete(vec![])),
            });
        }
        for copy in 0..2 {
            let mut assignments = vec![];
            for row in &parameters {
                let SlotDescriptor::Parameter(e) = row else {
                    panic!("physical parameter")
                };
                let SchemaState::Known(schema) = &e.schema else {
                    panic!("known physical parameter")
                };
                assert_eq!(
                    e.id.declaration,
                    SlotOwnerDefId::Gem(input.physical_gem.clone())
                );
                let value = match &schema.value {
                    ValueSchema::Boolean => ParameterValue::Boolean(false),
                    ValueSchema::Quantity(range) => ParameterValue::Quantity(
                        FiniteQuantity::new(0., range.minimum.unit().clone()).unwrap(),
                    ),
                    _ => panic!("unexpected physical parameter type"),
                };
                assignments.push(ParameterAssignment {
                    slot: e.id.clone(),
                    value,
                });
            }
            f.build.gems.push(GemInstance {
                id: occurrence(20 + copy),
                definition: input.physical_gem.clone(),
                level: 17 + copy as u16,
                quality: None,
                parameters: assignments,
            });
            f.build.skills.push(SkillUse {
                id: occurrence(30 + copy),
                source: AuthoredSkillSource::Gem(occurrence(20 + copy)),
                enabled: true,
                scope: LoadoutScope::Shared,
                parameters: None,
            });
            for set in 0..2 {
                f.queries.requests.push(MetricRequest {
                    id: QueryId::new(format!("copy-{copy}-set-{set}")).unwrap(),
                    metric: namespace(&base::def::<MetricDefinition>("requested")),
                    target: MetricTarget::Action(Box::new(action(copy as usize, set))),
                });
            }
        }
        Self { f }
    }
    fn schema(&self) -> OwnedDefinitionSchemaPackage {
        OwnedDefinitionSchemaPackage::new(self.f.schema.clone(), Default::default()).unwrap()
    }
    fn binding(&self) -> DefinitionBindingReport {
        bind_owned_request(&self.schema(), &self.f.request(), BindingLimits::default()).unwrap()
    }
    fn compile(&self, operations: &str) -> Result<Plan> {
        let schema = Arc::new(self.schema());
        let rules = Arc::new(
            CompiledRulePackage::compile(
                &RulePackageInput {
                    ordered_contributions: None,
                    schema_version: OWNED_RULE_PACKAGE_VERSION,
                    namespace: schema.input().namespace.clone(),
                    release: key("finite-ice-nova-topology"),
                    semantics_version: key("topology-only"),
                    operations_version: key(operations),
                    definitions: schema.identity().clone(),
                    owners: self.f.owners.clone(),
                    tables: vec![],
                    receivers: DeclaredSet::complete(vec![]),
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
                    release: key("no-numerical-routing"),
                    definitions: schema.identity().clone(),
                    outputs: self.f.routes.clone(),
                },
                schema.as_ref(),
                Default::default(),
            )
            .unwrap(),
        );
        OwnedEffectPlan::compile(
            Arc::new(self.f.request()),
            schema,
            rules,
            routing,
            Default::default(),
        )
    }
    fn plan(&self) -> Plan {
        self.compile(OWNED_RULE_OPERATIONS_V15).unwrap()
    }
}

#[test]
fn actual_stat_sets_bind_to_one_generated_skill_without_alias_skills() {
    let input = inputs();
    assert_eq!(input.schema_version, 1);
    assert_eq!(
        input
            .stat_sets
            .iter()
            .map(|s| s.source_index)
            .collect::<Vec<_>>(),
        [1, 2]
    );
    let world = World::new(true);
    let schema = world.schema();
    let request = world.f.request();
    let resolver =
        OwnedOccurrenceResolver::new(&schema, &request, BindingLimits::default()).unwrap();
    let mut selections = BTreeSet::new();
    for copy in 0..2 {
        for set in 0..2 {
            let selection = action(copy, set);
            let resolved = resolver.action(&selection).unwrap();
            assert_eq!(resolved.schema(), SchemaBindingStatus::Valid);
            assert_eq!(resolved.status(), SelectorBindingStatus::PendingResolution);
            let found = resolved.value().unwrap();
            assert_eq!(found.selection(), &selection);
            assert_eq!(found.expected_actor(), &ActorKey::Player);
            let ProviderExposure::Skill {
                key: generated,
                owner,
                ..
            } = found.provider().exposure()
            else {
                panic!("primary supply must expose one exact generated Skill")
            };
            assert_eq!(owner, &SlotOwnerDefId::Skill(input.primary_skill.clone()));
            assert_eq!(
                generated,
                &GeneratedSkillKey {
                    provider: provider(copy),
                    slot: input.primary_supply.clone()
                }
            );
            assert!(selections.insert(selection));
        }
        assert_eq!(action(copy, 0).action, action(copy, 1).action);
        assert_ne!(action(copy, 0).stat_set, action(copy, 1).stat_set);
    }
    assert_eq!(selections.len(), 4);
    assert_ne!(action(0, 0).action.provider, action(1, 0).action.provider);
    assert_eq!(world.f.build.skills.len(), 2);
    assert_eq!(world.f.build.gems.len(), 2);
    assert!(
        world
            .f
            .build
            .skills
            .iter()
            .all(|s| matches!(s.source, AuthoredSkillSource::Gem(_)))
    );
}

#[test]
fn disabled_physical_copy_makes_both_queries_unavailable_without_affecting_other_copy() {
    let mut world = World::new(true);
    world.f.build.skills[0].enabled = false;
    let bound = world.binding();
    assert_eq!(bound.schema(), SchemaBindingStatus::Valid);
    assert_eq!(bound.queries().len(), 4);
    for (index, query) in bound.queries().iter().enumerate() {
        assert_eq!(
            query.selector,
            if index < 2 {
                SelectorBindingStatus::Unavailable
            } else {
                SelectorBindingStatus::PendingResolution
            }
        );
    }
    let schema = world.schema();
    let request = world.f.request();
    let resolver =
        OwnedOccurrenceResolver::new(&schema, &request, BindingLimits::default()).unwrap();
    for set in 0..2 {
        let disabled = resolver.action(&action(0, set)).unwrap();
        assert_eq!(disabled.status(), SelectorBindingStatus::Unavailable);
        assert!(disabled.value().is_none());
        assert!(
            disabled
                .issues()
                .iter()
                .any(|i| i.code == BindingIssueCode::DisabledProvider)
        );
        assert!(resolver.action(&action(1, set)).unwrap().value().is_some());
    }
    let plan = world.plan();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(report.effects.iter().all(|e| e.key.invocation.origin
        != RuleOrigin::Provider {
            provider: provider(0)
        }));
    assert_eq!(report.effects.len(), 1);
}

#[test]
fn foreign_alternative_is_invalid_and_unsupplied_query_output_is_unavailable() {
    let world = World::new(true);
    let schema = world.schema();
    let request = world.f.request();
    let resolver =
        OwnedOccurrenceResolver::new(&schema, &request, BindingLimits::default()).unwrap();
    let mut wrong_set = action(0, 0);
    wrong_set.stat_set = namespace(&base::def::<ActionStatSetDefinition>("set"));
    let mut wrong_output = action(0, 0);
    wrong_output.action.output = namespace(&base::output());
    let invalid = resolver.action(&wrong_set).unwrap();
    assert_eq!(invalid.schema(), SchemaBindingStatus::Invalid);
    assert!(invalid.value().is_none());
    assert!(invalid.issues().iter().any(|i| {
        i.class == IssueClass::Invalid
            && i.site.facet == BindingFacet::StatSet
            && i.code == BindingIssueCode::NotDeclared
    }));

    // Historical queries may reference a known output that this occurrence no
    // longer supplies. This is unavailable, while an incompatible alternative
    // on an actually offered output above is invalid. Neither falls back.
    let absent = resolver.action(&wrong_output).unwrap();
    assert_eq!(absent.schema(), SchemaBindingStatus::Valid);
    assert_eq!(absent.status(), SelectorBindingStatus::Unavailable);
    assert!(absent.value().is_none());
    assert!(absent.issues().iter().any(|i| {
        i.class == IssueClass::Unavailable
            && i.site.facet == BindingFacet::Output
            && i.code == BindingIssueCode::NotDeclared
    }));
}

#[test]
fn declared_topology_can_be_resolved_without_fabricating_requested_actions_or_damage() {
    let mut world = World::new(true);
    world.f.queries.requests.clear();
    let schema = world.schema();
    let request = world.f.request();
    let resolver =
        OwnedOccurrenceResolver::new(&schema, &request, BindingLimits::default()).unwrap();
    for set in 0..2 {
        assert!(resolver.action(&action(0, set)).unwrap().value().is_some());
    }
    assert!(world.f.request().queries().input().requests.is_empty());
    let plan = world.plan();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert_eq!(report.effects.len(), 2);
    for copy in 0..2 {
        let expected = PlanValueKey::Grant {
            provider: provider(copy),
            slot: inputs().entering_grant.clone(),
        };
        let grant = report
            .values
            .iter()
            .find(|row| row.key == expected)
            .unwrap();
        assert_eq!(
            grant.value,
            EffectValue::Known {
                value: ParameterValue::Boolean(true)
            }
        );
    }
    assert!(
        report
            .values
            .iter()
            .all(|row| matches!(row.key, PlanValueKey::Grant { .. }))
    );
}

#[test]
fn unmodified_packet_preserves_partial_input_and_mechanics_and_requires_readiness() {
    let migration = asset("migration.json");
    assert_eq!(
        migration["contract"]["operations_version"],
        OWNED_RULE_OPERATIONS_V17
    );
    assert!(migration.get("evaluation").is_none());
    let world = World::new(false);
    for row in migration["schema"].as_array().unwrap() {
        match row["kind"].as_str().unwrap() {
            "definition" => {
                let expected: DefinitionDescriptor =
                    serde_json::from_value(row["value"].clone()).unwrap();
                assert!(world.f.schema.definitions.contains(&expected));
            }
            "slot" => {
                let expected: SlotDescriptor =
                    serde_json::from_value(row["value"].clone()).unwrap();
                assert!(world.f.schema.slots.contains(&expected));
            }
            _ => unreachable!(),
        }
    }
    assert_eq!(world.binding().schema(), SchemaBindingStatus::Unresolved);
    let schema = world.schema();
    let request = world.f.request();
    let resolver =
        OwnedOccurrenceResolver::new(&schema, &request, BindingLimits::default()).unwrap();
    for set in 0..2 {
        let result = resolver.action(&action(0, set)).unwrap();
        assert_eq!(result.status(), SelectorBindingStatus::Unresolved);
        assert!(result.value().is_none());
    }
    assert!(
        matches!(world.compile(OWNED_RULE_OPERATIONS_V17), Err(PlanError::Invalid(message)) if message == "operation v16 requires checked readiness stages")
    );
    let plan = world.plan();
    assert!(!plan.gaps().is_empty());
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(!report.gaps.is_empty());
    assert!(
        report
            .values
            .iter()
            .all(|row| !matches!(row.key, PlanValueKey::Stat { .. }))
    );
}

#[test]
fn topology_only_plans_reuse_scratch_and_private_rayon_workers_without_copy_leakage() {
    let a = World::new(true).plan();
    let mut disabled = World::new(true);
    disabled.f.build.skills[0].enabled = false;
    let b = disabled.plan();
    let expected = [
        a.evaluate(&mut a.new_scratch()).unwrap(),
        b.evaluate(&mut b.new_scratch()).unwrap(),
    ];
    assert_ne!(a.identity(), b.identity());
    let mut scratch = a.new_scratch();
    for (plan, report) in [(&a, &expected[0]), (&b, &expected[1]), (&a, &expected[0])] {
        assert!(
            plan.evaluate(&mut scratch).unwrap() == *report,
            "reused topology report changed"
        );
    }
    let results: Vec<_> = (0..32)
        .into_par_iter()
        .map_init(
            || a.new_scratch(),
            |scratch, index| {
                let which = index % 2;
                let plan = if which == 0 { &a } else { &b };
                (which, plan.evaluate(scratch).unwrap())
            },
        )
        .collect();
    for (which, report) in results {
        assert!(
            report == expected[which],
            "private worker topology report changed"
        );
    }
}
