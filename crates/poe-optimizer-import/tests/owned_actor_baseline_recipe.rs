//! Native component execution over the full acquired catalog. These fixtures do
//! not certify original build coverage or the unconverted actor mechanics.
use poe_optimizer_core::{
    owned_build::{DeclaredSlot, ParameterValue},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition, RuleFact};
use poe_optimizer_import::{
    owned_actor_baseline_recipe::*, owned_actor_baselines::*, owned_mapping::*, owned_recipe::*,
};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/owned/poe2/3887ae68")
}
fn integer(n: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(n).unwrap())
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn subject(actor: &DeclaredSlot<ActorSlotDefId>) -> SchemaSubject {
    SchemaSubject::Slot(SlotAddress::Actor(actor.clone()))
}
fn stat(
    registry: &mut OwnedIdRegistry,
    recipe: &mut OwnedRecipeInput,
    value: ComputedValueType,
) -> StatDefId {
    let id: StatDefId = registry.allocate_definition().unwrap();
    recipe
        .schema
        .definitions
        .push(DefinitionDescriptor::Stat(DefinitionEntry {
            id: id.clone(),
            schema: SchemaState::Known(StatSchema {
                value,
                targets: vec![RuleEntityKind::Actor],
            }),
        }));
    id
}
fn rule_owner(subject: SchemaSubject) -> DefinitionRules {
    DefinitionRules {
        owner: subject.clone(),
        programs: DeclaredSet::partial(
            vec![],
            vec![SchemaGap {
                subject,
                facet: SchemaFacet::GameRules,
                code: key("actor-mechanics-unconverted"),
            }],
        ),
    }
}
struct Fixture {
    recipe: OwnedRecipeInput,
    catalog: ActorBaselineCatalog,
    bytes: Vec<u8>,
    policy: ActorBaselinePolicy,
    time: StatDefId,
    damage: StatDefId,
    level: StatDefId,
    unit: UnitDefId,
}
impl Fixture {
    fn new() -> Self {
        let mut recipe: OwnedRecipeInput =
            serde_json::from_slice(&fs::read(root().join("current/recipe.json")).unwrap()).unwrap();
        let bytes = fs::read(root().join("actor-baselines/catalog.json")).unwrap();
        let catalog: ActorBaselineCatalog = serde_json::from_slice(&bytes).unwrap();
        let (actor, mut other_schema) = recipe
            .schema
            .slots
            .iter()
            .find_map(|s| {
                if let SlotDescriptor::Actor(a) = s {
                    Some((a.id.clone(), a.clone()))
                } else {
                    None
                }
            })
            .unwrap();
        let parameter=recipe.schema.slots.iter().find_map(|s|if let SlotDescriptor::Parameter(p)=s && p.id.declaration==actor.declaration && matches!(&p.schema,SchemaState::Known(s) if matches!(s.value,ValueSchema::Integer(_))) {Some(p.id.clone())}else{None}).unwrap();
        let mut registry =
            OwnedIdRegistry::new(recipe.registry.clone(), Default::default()).unwrap();
        let other = registry
            .allocate_slot::<ActorSlotDefinition>(actor.declaration.clone())
            .unwrap();
        other_schema.id = other.clone();
        recipe
            .schema
            .slots
            .push(SlotDescriptor::Actor(other_schema));
        let SlotOwnerDefId::Skill(skill) = &actor.declaration else {
            panic!()
        };
        for descriptor in &mut recipe.schema.definitions {
            if let DefinitionDescriptor::Skill(row) = descriptor
                && &row.id == skill
                && let SchemaState::Known(s) = &mut row.schema
            {
                s.declarations.actors.members.push(other.clone());
            }
        }
        recipe.rules.owners.push(rule_owner(subject(&other)));
        let unit: UnitDefId = registry.allocate_definition().unwrap();
        recipe
            .schema
            .definitions
            .push(DefinitionDescriptor::Unit(DefinitionEntry {
                id: unit.clone(),
                schema: SchemaState::Known(UnitSchema {
                    dimension: UnitDimension::Time,
                }),
            }));
        let damage_unit: UnitDefId = registry.allocate_definition().unwrap();
        recipe
            .schema
            .definitions
            .push(DefinitionDescriptor::Unit(DefinitionEntry {
                id: damage_unit.clone(),
                schema: SchemaState::Known(UnitSchema {
                    dimension: UnitDimension::Damage,
                }),
            }));
        let time = stat(
            &mut registry,
            &mut recipe,
            ComputedValueType::Quantity { unit: unit.clone() },
        );
        let damage = stat(
            &mut registry,
            &mut recipe,
            ComputedValueType::Quantity {
                unit: damage_unit.clone(),
            },
        );
        let level = stat(&mut registry, &mut recipe, ComputedValueType::Integer);
        let flag = stat(&mut registry, &mut recipe, ComputedValueType::Boolean);
        let mut bindings = vec![];
        for (index, (actor, profile, curve)) in [
            (actor, "RaisedSkeletonSniper", ActorDamageCurve::Allied),
            (other, "SandDjinn", ActorDamageCurve::Hostile),
        ]
        .into_iter()
        .enumerate()
        {
            bindings.push(ActorBaselineBinding {
                profile: profile.into(),
                actor,
                program: key("baseline"),
                fields: vec![
                    ActorScalarBinding {
                        field: ActorBaselineField::AttackTime,
                        target: ActorScalarTarget::Quantity {
                            stat: time.clone(),
                            unit: unit.clone(),
                        },
                        when_absent: ActorFactAbsence::Reject,
                    },
                    ActorScalarBinding {
                        field: ActorBaselineField::Hostile,
                        target: ActorScalarTarget::Boolean { stat: flag.clone() },
                        when_absent: ActorFactAbsence::Omit,
                    },
                ],
                curves: vec![ActorCurveBinding {
                    curve,
                    actor_level: level.clone(),
                    table: key(&format!("curve-{index}")),
                    stat: damage.clone(),
                    unit: damage_unit.clone(),
                }],
                summon_level: Some(ActorSummonLevelBinding {
                    parameter: parameter.clone(),
                    stat: level.clone(),
                    table: key("ordinary-levels"),
                    program: key(&format!("project-level-{index}")),
                }),
            });
        }
        recipe.registry = registry.input().clone();
        let policy = ActorBaselinePolicy {
            schema_version: 1,
            version: key("injected-actors"),
            catalog_sha256: hash(&bytes),
            source: catalog.source.clone(),
            bindings,
        };
        let mut f = Self {
            recipe,
            catalog,
            bytes,
            policy,
            time,
            damage,
            level,
            unit,
        };
        f.rebind();
        f
    }
    fn rebind(&mut self) {
        let schema =
            OwnedDefinitionSchemaPackage::new(self.recipe.schema.clone(), Default::default())
                .unwrap();
        self.recipe.rules.definitions = schema.identity().clone();
        self.recipe.routing.definitions = schema.identity().clone();
    }
    fn repin(&mut self) {
        self.bytes = serde_json::to_vec(&self.catalog).unwrap();
        self.policy.catalog_sha256 = hash(&self.bytes);
    }
    fn base(&self) -> StagedOwnedRecipe {
        assemble_owned_recipe(self.recipe.clone(), Default::default()).unwrap()
    }
    fn compile(&self) -> Result<StagedActorBaselineRecipe, ActorBaselineRecipeError> {
        compile_owned_actor_baselines(&self.base(), &self.bytes, &self.policy, Default::default())
    }
}
fn evaluated(
    out: &OwnedRecipeInput,
    owner: &SchemaSubject,
    program: &str,
    facts: &[RuleFact],
) -> Vec<(RuleEffectKind, EffectDisposition)> {
    let recipe = assemble_owned_recipe(out.clone(), Default::default()).unwrap();
    let compiled =
        CompiledRulePackage::compile(recipe.rules().input(), recipe.schema(), Default::default())
            .unwrap();
    let mut scratch = compiled.new_scratch();
    let first = compiled
        .evaluate(owner, &key(program), facts, recipe.schema(), &mut scratch)
        .unwrap();
    assert_eq!(
        first,
        compiled
            .evaluate(owner, &key(program), facts, recipe.schema(), &mut scratch)
            .unwrap()
    );
    first
        .effects
        .into_iter()
        .map(|e| (e.effect, e.disposition))
        .collect()
}
fn quantity_value(results: &[(RuleEffectKind, EffectDisposition)], stat: &StatDefId) -> f64 {
    results
        .iter()
        .find_map(|(e, d)| {
            if let RuleEffectKind::Derive { stat: s, .. } = e
                && s == stat
                && let EffectDisposition::Applied {
                    value: ParameterValue::Quantity(v),
                } = d
            {
                Some(v.value())
            } else {
                None
            }
        })
        .unwrap()
}
#[test]
fn actual_profiles_compile_scalar_zero_and_explicit_different_curves_to_native_rules() {
    let f = Fixture::new();
    let out = f.compile().unwrap();
    assert_eq!(out.receipt.actors, 2);
    assert_eq!(out.receipt.scalar_facts, 2);
    assert_eq!(out.receipt.absent_facts, 2);
    assert_eq!(out.receipt.curve_outputs, 2);
    assert_eq!(out.receipt.level_projections, 2);
    assert_eq!(out.extension.tables.len(), 3);
    assert_eq!(out.successor.registry, f.base().registry().input().clone());
    assert_eq!(out.successor.schema, f.base().schema().input().clone());
    assert_eq!(out.successor.routing, f.base().routing().input().clone());
    assert_eq!(
        out.successor.rules.operations_version,
        f.recipe.rules.operations_version
    );
    for (i, time, curve) in [
        (0, 1.5, &f.catalog.allied_damage),
        (1, 0.0, &f.catalog.hostile_damage),
    ] {
        let rows = evaluated(
            &out.successor,
            &subject(&f.policy.bindings[i].actor),
            "baseline",
            &[RuleFact {
                read: key("level-0"),
                value: integer(40),
            }],
        );
        assert_eq!(quantity_value(&rows, &f.time), time);
        assert_eq!(quantity_value(&rows, &f.damage), curve.rows[39]);
        let owner = out
            .successor
            .rules
            .owners
            .iter()
            .find(|o| o.owner == subject(&f.policy.bindings[i].actor))
            .unwrap();
        assert!(!owner.programs.is_complete());
    }
    assert_eq!(out.receipt.remaining_coverage[1].unconverted_modifiers, 3);
}
#[test]
fn exact_parameter_projection_uses_input_level_and_keeps_out_of_domain_unresolved() {
    let f = Fixture::new();
    let out = f.compile().unwrap();
    let SlotOwnerDefId::Skill(owner) = &f.policy.bindings[0].actor.declaration else {
        panic!()
    };
    let owner = SchemaSubject::Definition(owner.address());
    for (level, expected) in [(1, 2), (20, 40), (40, 80)] {
        let rows = evaluated(
            &out.successor,
            &owner,
            "project-level-0",
            &[RuleFact {
                read: key("level"),
                value: integer(level),
            }],
        );
        assert!(
            matches!(&rows[0],(RuleEffectKind::ProjectActorStat{actor,stat,..},EffectDisposition::Applied{value}) if actor==&f.policy.bindings[0].actor && stat==&f.level && *value==integer(expected))
        );
    }
    // The creating parameter itself declares 1..40: supplied out-of-range facts
    // are invalid input, not lookup-domain results. Actor-stat reads below have
    // only an Integer type and therefore exercise the table-domain boundary.
    let checked = assemble_owned_recipe(out.successor.clone(), Default::default()).unwrap();
    let compiled = CompiledRulePackage::compile(
        checked.rules().input(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    for value in [0, 41] {
        assert!(
            compiled
                .evaluate(
                    &owner,
                    &key("project-level-0"),
                    &[RuleFact {
                        read: key("level"),
                        value: integer(value)
                    }],
                    checked.schema(),
                    &mut compiled.new_scratch()
                )
                .is_err()
        );
    }
    let rows = evaluated(&out.successor, &owner, "project-level-0", &[]);
    assert!(matches!(rows[0].1, EffectDisposition::Unresolved { .. }));
    for level in [0, 101] {
        let rows = evaluated(
            &out.successor,
            &subject(&f.policy.bindings[0].actor),
            "baseline",
            &[RuleFact {
                read: key("level-0"),
                value: integer(level),
            }],
        );
        assert!(rows.iter().any(
            |(e, d)| matches!(e,RuleEffectKind::Derive{stat,..} if stat==&f.damage)
                && !matches!(d, EffectDisposition::Applied { .. })
        ));
    }
}
#[test]
fn unchanged_append_is_canonical_and_data_only_changes_affect_new_compilation() {
    let mut f = Fixture::new();
    let first = f.compile().unwrap();
    let base = assemble_owned_recipe(first.successor.clone(), Default::default()).unwrap();
    let second =
        compile_owned_actor_baselines(&base, &f.bytes, &f.policy, Default::default()).unwrap();
    assert_eq!(second.successor, first.successor);
    assert_eq!(second.receipt.extension.appended_programs, 0);
    assert_eq!(second.receipt.extension.appended_tables, 0);
    f.policy.bindings.reverse();
    f.policy
        .bindings
        .iter_mut()
        .for_each(|b| b.fields.reverse());
    let reordered =
        compile_owned_actor_baselines(&base, &f.bytes, &f.policy, Default::default()).unwrap();
    assert_eq!(reordered.successor, first.successor);
    f.catalog
        .profiles
        .iter_mut()
        .find(|p| p.key == "RaisedSkeletonSniper")
        .unwrap()
        .attack_time = Some(2.0);
    f.repin();
    assert!(compile_owned_actor_baselines(&base, &f.bytes, &f.policy, Default::default()).is_err());
    let fresh = f.compile().unwrap();
    let actor = &f
        .policy
        .bindings
        .iter()
        .find(|b| b.profile == "RaisedSkeletonSniper")
        .unwrap()
        .actor;
    assert_eq!(
        quantity_value(
            &evaluated(&fresh.successor, &subject(actor), "baseline", &[]),
            &f.time
        ),
        2.0
    );
}
#[test]
fn wrong_units_scopes_ownership_duplicates_and_absence_do_not_gain_coverage() {
    for case in 0..8 {
        let mut f = Fixture::new();
        match case {
            0 => f.policy.bindings[0].fields[1].when_absent = ActorFactAbsence::Reject,
            1 => {
                f.policy.bindings[0].fields[0].target = ActorScalarTarget::Quantity {
                    stat: f.time.clone(),
                    unit: f.policy.bindings[0].curves[0].unit.clone(),
                }
            }
            2 => {
                let b = f.policy.bindings[0].clone();
                f.policy.bindings.push(b);
            }
            3 => {
                f.policy.bindings[0].actor.declaration = SlotOwnerDefId::Skill(
                    SkillDefId::parse(f.recipe.schema.namespace.clone(), "missing").unwrap(),
                )
            }
            4 => {
                let other = f.policy.bindings[1].actor.slot.clone();
                f.policy.bindings[0].actor.slot = other;
            }
            5 => {
                f.policy.bindings[0]
                    .summon_level
                    .as_mut()
                    .unwrap()
                    .parameter
                    .declaration = SlotOwnerDefId::Skill(
                    SkillDefId::parse(f.recipe.schema.namespace.clone(), "missing").unwrap(),
                )
            }
            6 => f.policy.bindings[0].curves[0].table = key("ordinary-levels"),
            _ => f.policy.bindings[0].profile = "not-in-the-catalog".into(),
        }
        assert!(f.compile().is_err(), "case {case}");
    }
}
#[test]
fn collisions_and_complete_owners_cannot_be_replaced_or_closed() {
    for complete in [false, true] {
        let mut f = Fixture::new();
        let owner = f
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == subject(&f.policy.bindings[0].actor))
            .unwrap();
        if complete {
            owner.programs.closure = SchemaClosure::Complete;
        } else {
            owner.programs.members.push(RuleProgram {
                id: key("competing"),
                context: RuleEntityKind::Actor,
                reads: vec![],
                nodes: vec![RuleNode {
                    id: key("n"),
                    expression: RuleExpression::Literal {
                        value: ParameterValue::Quantity(
                            FiniteQuantity::new(99.0, f.unit.clone()).unwrap(),
                        ),
                    },
                }],
                effects: vec![RuleEffect {
                    id: key("n"),
                    when: None,
                    effect: RuleEffectKind::Derive {
                        entity: RuleEntity::Current,
                        stat: f.time.clone(),
                        value: key("n"),
                    },
                }],
            });
        }
        assert!(f.compile().is_err());
    }
    let mut f = Fixture::new();
    let existing=f.recipe.rules.owners.iter_mut().find(|o|matches!(&o.owner,SchemaSubject::Definition(DefinitionAddress::Skill(id)) if SlotOwnerDefId::Skill(id.clone())==f.policy.bindings[0].actor.declaration)).unwrap();
    for program in &mut existing.programs.members {
        for effect in &mut program.effects {
            if let RuleEffectKind::ProjectActorStat { stat, .. } = &mut effect.effect
                && program.id.as_str() == "ordinary-population-inputs"
                && effect.id.as_str() == "project-actor-level"
            {
                *stat = f.level.clone();
            }
        }
    }
    assert!(f.compile().is_err());
}
#[test]
fn exact_catalog_binding_and_compiler_budgets_are_checked() {
    let mut f = Fixture::new();
    f.policy.catalog_sha256 = "0".repeat(64);
    assert!(f.compile().is_err());
    let mut f = Fixture::new();
    f.policy.source.revision = "other".into();
    assert!(f.compile().is_err());
    let f = Fixture::new();
    let base = f.base();
    for limits in [
        ActorBaselineRecipeLimits {
            max_work: 1,
            ..Default::default()
        },
        ActorBaselineRecipeLimits {
            max_catalog_bytes: 1,
            ..Default::default()
        },
        ActorBaselineRecipeLimits {
            max_profiles: 648,
            ..Default::default()
        },
        ActorBaselineRecipeLimits {
            max_rows: 99,
            ..Default::default()
        },
        ActorBaselineRecipeLimits {
            max_bindings: 1,
            ..Default::default()
        },
    ] {
        assert!(compile_owned_actor_baselines(&base, &f.bytes, &f.policy, limits).is_err());
    }
}
