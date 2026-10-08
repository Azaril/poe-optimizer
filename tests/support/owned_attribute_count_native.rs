//! Actual published contribution bodies in an unpublished finite native domain.
//! Only selected class bases and attribute-producing passive occurrences are
//! admitted here. The original identifiers, choices and loadout scopes survive;
//! equipment, ascendancies, implicit passives and external effects are excluded.
//! This is a contribution census, never a final-attribute or build-completeness
//! certificate. The production Partial closures are checked separately below.
use super::{empty_support, family, release};
use poe_optimizer_core::{
    build_identity::WeaponLoadoutId, owned_build::*, owned_definitions::*, owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::{
    owned_plan::*,
    owned_rules::{CompiledRulePackage, EffectDisposition, NumericalFailure, RuleFact},
};
use poe_optimizer_import::owned_recipe::{OwnedRecipeInput, assemble_owned_recipe};
use rayon::prelude::*;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{collections::BTreeSet, fs, path::PathBuf, sync::OnceLock};

fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap()
}
fn def<K: DefinitionDomain>(n: u64) -> DefId<K> {
    DefId::parse(ns(), format!("def.{n:016x}")).unwrap()
}
fn decode<T: DeserializeOwned>(v: &Value) -> T {
    serde_json::from_value(v.clone()).unwrap()
}
fn subject<I: SchemaDefinitionId>(id: I) -> SchemaSubject {
    SchemaSubject::Definition(id.address())
}
fn known(v: &Value) -> &Value {
    assert_eq!(v["kind"], "known");
    &v["value"]
}
fn referenced_keys(v: &Value, keys: &mut BTreeSet<String>) {
    match v {
        Value::Object(o) => {
            if o.contains_key("namespace")
                && let Some(k) = o.get("key").and_then(Value::as_str)
            {
                keys.insert(k.into());
            }
            for x in o.values() {
                referenced_keys(x, keys);
            }
        }
        Value::Array(a) => {
            for x in a {
                referenced_keys(x, keys);
            }
        }
        _ => {}
    }
}
#[derive(Clone)]
struct Transition {
    owner: SchemaSubject,
    before: RuleProgram,
    after: RuleProgram,
}
struct Source {
    recipe: OwnedRecipeInput,
    actual_owners: Vec<DefinitionRules>,
    transitions: Vec<Transition>,
    builds: Vec<BuildInput>,
    edited_node: PassiveNodeDefId,
    projection_recipe: OwnedRecipeInput,
}
fn source() -> &'static Source {
    static SOURCE: OnceLock<Source> = OnceLock::new();
    SOURCE.get_or_init(|| {
        let p = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ATTRIBUTE_COUNT_RELEASE")
                .expect("verified attribute Count release"),
        );
        let inventory = release::inventory(&p);
        let endpoint = release::load(&p);
        family::assert_programs(&endpoint.input().recipe.rules);
        let rows: Value = family::read("count-programs.json");
        let transitions: Vec<_> = rows["programs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                let t = Transition {
                    owner: decode(&r["owner"]),
                    before: decode(&r["before"]),
                    after: decode(&r["after"]),
                };
                let actual = endpoint
                    .input()
                    .recipe
                    .rules
                    .owners
                    .iter()
                    .find(|o| o.owner == t.owner)
                    .unwrap();
                assert_eq!(actual.programs.closure, decode(&r["closure"]));
                assert_eq!(
                    actual
                        .programs
                        .members
                        .iter()
                        .filter(|v| **v == t.after)
                        .count(),
                    1
                );
                t
            })
            .collect();
        assert_eq!(transitions.len(), 368);
        let read = |name: &str| -> Value {
            serde_json::from_slice(&fs::read(p.parent().unwrap().join(name)).unwrap()).unwrap()
        };
        let mut used = Vec::new();
        let builds = (1..=5)
            .map(|i| {
                let document = read(&format!("original-{i:02}/draft.json"));
                let d = &document["draft"];
                let selected = read(&format!("selected-{i:02}.json"));
                let selected = &selected["build"];
                let character = d["character_presets"]["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["id"] == selected["character"])
                    .unwrap();
                let class: ClassDefId = decode(known(&character["class"]));
                let class_owner = subject(class.clone());
                if !used.contains(&class_owner) {
                    used.push(class_owner);
                }
                let allocations = d["allocation_presets"]["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["id"] == selected["allocations"])
                    .unwrap();
                assert_eq!(allocations["allocations"]["completion"]["kind"], "complete");
                let ids = allocations["allocations"]["members"].as_array().unwrap();
                let allocations = d["allocations"]["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|r| ids.contains(&r["id"]))
                    .filter_map(|r| {
                        let node: PassiveNodeDefId = decode(known(&r["node"]));
                        let owner = subject(node.clone());
                        let t = transitions.iter().find(|t| t.owner == owner)?;
                        assert!(matches!(
                            t.after.id.as_str(),
                            "attribute-choice" | "passive-view"
                        ));
                        if !used.contains(&owner) {
                            used.push(owner);
                        }
                        assert_eq!(r["choices"]["completion"]["kind"], "complete");
                        Some(Allocation {
                            id: decode(&r["id"]),
                            node,
                            pool: decode(known(&r["pool"])),
                            scope: decode(known(&r["scope"])),
                            access: decode(&r["access"]),
                            choices: r["choices"]["members"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|c| ChoiceSelection {
                                    slot: decode(known(&c["slot"])),
                                    value: decode(known(&c["value"])),
                                })
                                .collect(),
                        })
                    })
                    .collect();
                BuildInput {
                    allocator: decode(&d["allocator"]),
                    revision: decode(&d["revision"]),
                    game_version: ns(),
                    character: CharacterSpec {
                        class,
                        ascendancy: None,
                        level: decode(known(&character["level"])),
                        rewards: vec![],
                    },
                    weapon_loadouts: decode(&d["weapon_loadouts"]["members"]),
                    active_weapon_loadout: decode(&selected["active_weapon_loadout"]),
                    allocations,
                    items: vec![],
                    gems: vec![],
                    equipment: vec![],
                    skills: vec![],
                    supports: vec![],
                    payload_links: vec![],
                    choices: vec![],
                    authored_support_order: Some(vec![]),
                    generated_inputs: Some(GeneratedSkillInputsV1 {
                        schema_version: 1,
                        bindings: vec![],
                    }),
                }
            })
            .collect::<Vec<_>>();
        let actual_owners: Vec<_> = endpoint
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .filter(|o| used.contains(&o.owner))
            .cloned()
            .collect();
        assert_eq!(actual_owners.len(), used.len());
        let mut keys = BTreeSet::new();
        referenced_keys(&json!(actual_owners), &mut keys);
        referenced_keys(&json!(builds), &mut keys);
        for t in &transitions {
            if used.contains(&t.owner) {
                referenced_keys(&json!(t.before), &mut keys);
            }
        }
        for n in [1, 2, 0x31d1, 0x295a, 0x1d2e, 0x1d2f, 0x1d30] {
            keys.insert(format!("def.{n:016x}"));
        }
        let mut recipe = endpoint.input().recipe.clone();
        recipe
            .schema
            .slots
            .retain(|s| keys.contains(s.address().key().as_str()));
        referenced_keys(&json!(recipe.schema.slots), &mut keys);
        // A retained Stat may only be named by a program, so its quantity unit
        // need not occur in any literal/read. Keep that exact schema dependency
        // before pruning definitions, including dependencies of the full actual
        // class owners restored by the Partial-owner negative control.
        for descriptor in &recipe.schema.definitions {
            if keys.contains(descriptor.address().key().as_str())
                && let DefinitionDescriptor::Stat(entry) = descriptor
                && let SchemaState::Known(schema) = &entry.schema
                && let ComputedValueType::Quantity { unit } = &schema.value
            {
                keys.insert(unit.key().as_str().to_owned());
            }
        }
        recipe
            .schema
            .definitions
            .retain(|d| keys.contains(d.address().key().as_str()));
        // The finite fixture has no topology, ascendancy or implicit providers.
        // Actual selected passive declaration/choice inventories remain intact.
        for d in &mut recipe.schema.definitions {
            match d {
                DefinitionDescriptor::Class(e) => {
                    let SchemaState::Known(s) = &mut e.schema else {
                        panic!()
                    };
                    s.ascendancies = DeclaredSet::complete(vec![]);
                    s.implicit_passives = DeclaredSet::complete(vec![]);
                }
                DefinitionDescriptor::PassiveNode(e) => {
                    let SchemaState::Known(s) = &mut e.schema else {
                        panic!()
                    };
                    s.adjacent = DeclaredSet::complete(vec![]);
                }
                DefinitionDescriptor::Encounter(e) => {
                    let SchemaState::Known(s) = &mut e.schema else {
                        panic!()
                    };
                    s.external_inputs = DeclaredSet::complete(vec![]);
                }
                DefinitionDescriptor::Stat(_)
                | DefinitionDescriptor::Unit(_)
                | DefinitionDescriptor::PointPool(_)
                | DefinitionDescriptor::Option(_) => {}
                _ => panic!(
                    "unexpected finite contribution dependency: {:?}",
                    d.address()
                ),
            }
        }
        recipe.rules.tables.clear();
        recipe.rules.receivers = DeclaredSet::complete(vec![]);
        recipe.rules.effect_applications = Some(DeclaredSet::complete(vec![]));
        assert!(recipe.rules.contribution_queries.is_none());
        recipe.routing.outputs.clear();
        recipe.rules.owners = recipe
            .schema
            .definitions
            .iter()
            .map(|d| {
                let owner = SchemaSubject::Definition(d.address());
                let programs = transitions
                    .iter()
                    .filter(|t| t.owner == owner && used.contains(&owner))
                    .map(|t| t.after.clone())
                    .collect();
                DefinitionRules {
                    owner,
                    programs: DeclaredSet::complete(programs),
                }
            })
            .collect();
        let tree = json!(endpoint.input().tree);
        let role = &tree["content"]["tokens"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["token"] == "15782")
            .unwrap()["role"];
        assert_eq!(role["kind"], "allocation");
        let edited_node = decode(&role["value"]["node"]);
        let projection_recipe = endpoint.input().recipe.clone();
        assert_eq!(inventory, release::inventory(&p));
        Source {
            recipe,
            actual_owners,
            transitions,
            builds,
            edited_node,
            projection_recipe,
        }
    })
}
#[derive(Clone)]
struct World {
    recipe: OwnedRecipeInput,
    build: BuildInput,
}
/// Reuse this explicitly finite fixture without copying its actual draft joins.
/// The immutable Count release is supplied through ATTRIBUTE_COUNT_RELEASE.
pub(super) fn finite_parts(index: usize) -> (OwnedRecipeInput, BuildInput) {
    (source().recipe.clone(), source().builds[index].clone())
}
pub(super) fn choice_control_node() -> PassiveNodeDefId {
    source().edited_node.clone()
}
impl World {
    fn new(index: usize) -> Self {
        let (recipe, build) = finite_parts(index);
        Self { recipe, build }
    }
    fn old(&self) -> Self {
        let mut old = self.clone();
        for owner in &mut old.recipe.rules.owners {
            for p in &mut owner.programs.members {
                *p = source()
                    .transitions
                    .iter()
                    .find(|t| t.owner == owner.owner && t.after.id == p.id)
                    .unwrap()
                    .before
                    .clone();
            }
        }
        old
    }
    fn plan(&self) -> OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage> {
        let scenario = ScenarioInput {
            game_version: ns(),
            enemy: EnemySpec {
                encounter: def(0x31d1),
                level: 20,
            },
            assumptions: vec![],
            usage: vec![],
        };
        empty_support::compile(&self.recipe, &self.build, &scenario, def(2)).unwrap()
    }
    fn report(&self) -> SupportEffectsReport {
        let p = self.plan();
        p.evaluate(&mut p.new_scratch()).unwrap()
    }
    fn evaluate(&self) -> OwnedEffectsReport {
        effects(self.report())
    }
}
fn effects(r: SupportEffectsReport) -> OwnedEffectsReport {
    let SupportEffectsOutcome::Evaluated { effects } = r.outcome else {
        panic!("{r:?}")
    };
    assert!(r.gaps.is_empty(), "{:?}", r.gaps);
    assert!(effects.gaps.is_empty());
    effects
}
fn active(a: &Allocation, loadout: WeaponLoadoutId) -> bool {
    match &a.scope {
        LoadoutScope::Shared => true,
        LoadoutScope::Selected { loadouts } => loadouts.contains(&loadout),
    }
}
fn attribute(stat: &StatDefId) -> Option<usize> {
    (0..3).find(|i| *stat == def(0x1d2e + *i as u64))
}
fn compare_cutover(w: &World) -> OwnedEffectsReport {
    let before = w.old().evaluate();
    let after = w.evaluate();
    let mut expected = Vec::new();
    for e in &before.effects {
        let BoundEffectTarget::Contribution { key } = &e.target else {
            panic!("contribution-only fixture")
        };
        assert_eq!(key.entity, ConcreteEntity::Actor(ActorKey::Player));
        if let Some(i) = attribute(&key.stat) {
            assert!(matches!(
                key.kind,
                ContributionKind::Add | ContributionKind::Increase
            ));
            for pass in 0..2 {
                let mut v = e.clone();
                let BoundEffectTarget::Contribution { key } = &mut v.target else {
                    unreachable!()
                };
                key.stat = def(0x331b + i as u64 + pass * 3);
                if pass == 1 {
                    v.key.effect = format!("{}-pass-two", v.key.effect.as_str())
                        .parse()
                        .unwrap();
                }
                if let EffectValue::Known {
                    value: ParameterValue::Integer(n),
                } = &v.value
                {
                    assert_eq!(key.kind, ContributionKind::Add);
                    v.value = EffectValue::Known {
                        value: ParameterValue::Quantity(
                            FiniteQuantity::new(n.get() as f64, def(0x295a)).unwrap(),
                        ),
                    };
                }
                expected.push(v);
            }
        } else {
            expected.push(e.clone());
        }
    }
    assert_eq!(after.effects.len(), expected.len());
    for e in &expected {
        assert_eq!(
            after.effects.iter().filter(|v| v.key == e.key).count(),
            1,
            "exact occurrence/effect"
        );
        assert_eq!(after.effects.iter().find(|v| v.key == e.key).unwrap(), e);
    }
    for e in &after.effects {
        if let BoundEffectTarget::Contribution { key } = &e.target {
            assert!(
                attribute(&key.stat).is_none(),
                "old integer attribute contribution survived"
            );
        }
    }
    assert!(!after.values.iter().any(|v| matches!(&v.key, PlanValueKey::Stat { stat, .. } if attribute(stat).is_some() || (0..6).any(|i| *stat == def(0x331b+i)))));
    after
}
fn census(report: &OwnedEffectsReport, pass: u64) -> [f64; 3] {
    let mut sums = [0.; 3];
    for e in &report.effects {
        let BoundEffectTarget::Contribution { key } = &e.target else {
            continue;
        };
        for (i, total) in sums.iter_mut().enumerate() {
            if key.stat == def(0x331b + pass * 3 + i as u64)
                && key.kind == ContributionKind::Add
                && let EffectValue::Known {
                    value: ParameterValue::Quantity(q),
                } = &e.value
            {
                assert_eq!(q.unit(), &def(0x295a));
                assert_eq!(q.value().fract(), 0.);
                *total += q.value();
            }
        }
    }
    sums
}
#[test]
#[ignore = "requires verified ATTRIBUTE_COUNT_RELEASE"]
fn all_five_actual_class_and_passive_streams_keep_occurrences_and_both_count_passes() {
    let expected_allocations = [28, 29, 24, 48, 22];
    let expected_active_choices = [25, 26, 21, 45, 22];
    for i in 0..5 {
        let w = World::new(i);
        assert_eq!(w.build.allocations.len(), expected_allocations[i]);
        assert_eq!(
            w.build
                .allocations
                .iter()
                .filter(|a| !a.choices.is_empty() && active(a, w.build.active_weapon_loadout))
                .count(),
            expected_active_choices[i]
        );
        let r = compare_cutover(&w);
        assert_eq!(census(&r, 0), census(&r, 1));
        for a in &w.build.allocations {
            let count = r.effects.iter().filter(|e| matches!(&e.key.invocation.origin, RuleOrigin::Provider { provider } if provider.root == ProviderRoot::Allocation(a.id) && provider.grant_path.is_empty())).count();
            assert_eq!(count > 0, active(a, w.build.active_weapon_loadout));
        }
        if i == 4 {
            assert_eq!(
                census(&r, 0),
                [27., 7., 105.],
                "raw class/passive Add census only"
            );
            let class = r.effects.iter().filter(|e| matches!(&e.key.invocation.origin, RuleOrigin::Provider { provider } if provider.root == ProviderRoot::Character)).collect::<Vec<_>>();
            assert_eq!(class.len(), 6);
            for (offset, amount) in [(0, 7.), (1, 7.), (2, 15.)] {
                assert!(class.iter().any(|e| matches!(&e.target, BoundEffectTarget::Contribution { key } if key.stat == def(0x331b+offset)) && e.value == EffectValue::Known { value: ParameterValue::Quantity(FiniteQuantity::new(amount, def(0x295a)).unwrap()) }));
            }
        }
    }
}
#[test]
#[ignore = "requires verified ATTRIBUTE_COUNT_RELEASE"]
fn exact_choice_edit_and_loadout_switch_rebind_each_actual_occurrence() {
    let mut w = World::new(4);
    let original = compare_cutover(&w);
    let a = w
        .build
        .allocations
        .iter_mut()
        .find(|a| a.node == choice_control_node())
        .unwrap();
    assert_eq!(a.choices.len(), 1);
    assert_eq!(a.choices[0].value, ParameterValue::Option(def(0x1bf4)));
    let edited = a.id;
    a.choices[0].value = ParameterValue::Option(def(0x1bf2));
    let changed = compare_cutover(&w);
    assert_eq!(census(&changed, 0), [22., 12., 105.]);
    for e in &original.effects {
        if !matches!(&e.key.invocation.origin, RuleOrigin::Provider { provider } if provider.root == ProviderRoot::Allocation(edited))
        {
            assert_eq!(changed.effects.iter().find(|v| v.key == e.key), Some(e));
        }
    }
    w.build
        .allocations
        .iter_mut()
        .find(|a| a.id == edited)
        .unwrap()
        .choices[0]
        .value = ParameterValue::Option(def(0x1bf4));
    assert_eq!(w.evaluate(), original);
    let mut other = World::new(1);
    let first = compare_cutover(&other);
    let old_loadout = other.build.active_weapon_loadout;
    other.build.active_weapon_loadout = *other
        .build
        .weapon_loadouts
        .iter()
        .find(|v| **v != old_loadout)
        .unwrap();
    let second = compare_cutover(&other);
    assert_ne!(census(&first, 0), census(&second, 0));
    for a in &other.build.allocations {
        let represented = second.effects.iter().any(|e| matches!(&e.key.invocation.origin, RuleOrigin::Provider { provider } if provider.root == ProviderRoot::Allocation(a.id)));
        assert_eq!(represented, active(a, other.build.active_weapon_loadout));
    }
}
#[test]
#[ignore = "requires verified ATTRIBUTE_COUNT_RELEASE"]
fn actual_partial_class_owners_still_refuse_complete_evaluation() {
    for i in 0..5 {
        let mut w = World::new(i);
        let owner = subject(w.build.character.class.clone());
        let actual = source()
            .actual_owners
            .iter()
            .find(|o| o.owner == owner)
            .unwrap();
        assert!(!actual.programs.is_complete());
        *w.recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == owner)
            .unwrap() = actual.clone();
        let r = w.report();
        assert!(
            r.gaps
                .iter()
                .any(|g| g.reason == PlanGapReason::PartialPrograms
                    && g.subject.as_ref() == Some(&owner))
        );
        assert_eq!(
            r.outcome,
            SupportEffectsOutcome::Unavailable {
                cause: EffectValue::Unresolved {
                    reason: PlanGapReason::IncompleteContributors,
                    read: None
                },
                input: None
            }
        );
    }
}
#[test]
#[ignore = "requires verified ATTRIBUTE_COUNT_RELEASE"]
fn count_reports_are_identical_with_fresh_reused_and_parallel_workers() {
    let a = World::new(4);
    let mut b = a.clone();
    b.build
        .allocations
        .iter_mut()
        .find(|v| v.node == choice_control_node())
        .unwrap()
        .choices[0]
        .value = ParameterValue::Option(def(0x1bf2));
    let pa = a.plan();
    let pb = b.plan();
    let va = a.evaluate();
    let vb = b.evaluate();
    assert_ne!(pa.identity(), pb.identity());
    let mut scratch = pa.new_scratch();
    for (p, expected) in [(&pa, &va), (&pb, &vb), (&pa, &va)] {
        assert_eq!(&effects(p.evaluate(&mut scratch).unwrap()), expected);
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let parallel: Vec<_> = pool.install(|| {
        (0..16)
            .into_par_iter()
            .map(|i| {
                let p = if i % 2 == 0 { &pa } else { &pb };
                effects(p.evaluate(&mut p.new_scratch()).unwrap())
            })
            .collect()
    });
    let expected: Vec<_> = (0..16)
        .map(|i| if i % 2 == 0 { va.clone() } else { vb.clone() })
        .collect();
    assert_eq!(parallel, expected);
}
#[test]
#[ignore = "requires verified ATTRIBUTE_COUNT_RELEASE"]
fn item_projection_keeps_quantization_missing_input_and_overflow_boundaries() {
    let checked =
        assemble_owned_recipe(source().projection_recipe.clone(), Default::default()).unwrap();
    let compiled = CompiledRulePackage::compile(
        checked.rules().input(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let mut scratch = compiled.new_scratch();
    let projections: Vec<_> = source()
        .transitions
        .iter()
        .filter(|t| t.after.id.as_str() == "contribute-player-attributes")
        .collect();
    assert_eq!(projections.len(), 4);
    for t in projections {
        let owner = checked
            .rules()
            .input()
            .owners
            .iter()
            .find(|o| o.owner == t.owner)
            .unwrap();
        assert!(!owner.programs.is_complete());
        for (input, expected) in [
            (10.9, 10.),
            (-10.1, -11.),
            (0., 0.),
            (BoundedInteger::MIN as f64, BoundedInteger::MIN as f64),
            (BoundedInteger::MAX as f64, BoundedInteger::MAX as f64),
        ] {
            let facts = [RuleFact {
                read: t.after.reads[0].id.clone(),
                value: ParameterValue::Quantity(FiniteQuantity::new(input, def(0x295a)).unwrap()),
            }];
            let r = compiled
                .evaluate(
                    &t.owner,
                    &t.after.id,
                    &facts,
                    checked.schema(),
                    &mut scratch,
                )
                .unwrap();
            assert_eq!(r.owner_programs_closure, owner.programs.closure);
            assert_eq!(r.effects.len(), t.after.effects.len());
            for e in r.effects {
                assert_eq!(
                    e.disposition,
                    EffectDisposition::Applied {
                        value: ParameterValue::Quantity(
                            FiniteQuantity::new(expected, def(0x295a)).unwrap()
                        )
                    }
                );
            }
        }
        let missing = compiled
            .evaluate(&t.owner, &t.after.id, &[], checked.schema(), &mut scratch)
            .unwrap();
        assert!(missing.effects.iter().all(|e| matches!(&e.disposition, EffectDisposition::Unresolved { input } if input.as_str() == "effective")));
        for input in [
            BoundedInteger::MIN as f64 - 1.,
            BoundedInteger::MAX as f64 + 1.,
        ] {
            let facts = [RuleFact {
                read: t.after.reads[0].id.clone(),
                value: ParameterValue::Quantity(FiniteQuantity::new(input, def(0x295a)).unwrap()),
            }];
            let r = compiled
                .evaluate(
                    &t.owner,
                    &t.after.id,
                    &facts,
                    checked.schema(),
                    &mut scratch,
                )
                .unwrap();
            assert!(r.effects.iter().all(|e| matches!(&e.disposition, EffectDisposition::NumericalError { node, reason: NumericalFailure::IntegerOverflow } if node.as_str() == "integer")));
        }
    }
}
