//! Exact class-to-existing-Player ownership cutover. All game coverage survives.
use super::{family, migration_preservation};
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_import::{
    owned_mapping::OwnedIdRegistry,
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub const KIND: &str = "shared-player-rule-ownership";
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/player-rule-ownership")
}
fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn decode<T: DeserializeOwned>(v: &Value) -> T {
    serde_json::from_value(v.clone()).unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    s.parse().unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn digest() -> OwnedContentDigest {
    let v: Vec<Value> = [
        "authoring.json",
        "bindings.json",
        "migration.json",
        "ownership.json",
        "source-vectors.json",
    ]
    .map(read)
    .into();
    digest_owned(KIND, &v, 1024 * 1024).unwrap()
}
#[derive(Clone, Deserialize)]
struct Replacement {
    before: DefinitionRules,
    after: DefinitionRules,
}
#[derive(Clone, Deserialize)]
struct Ownership {
    schema_version: u32,
    classes: Vec<Replacement>,
    actor: DefinitionRules,
    applicability: DeclaredSet<ExistingActorRuleApplication>,
}
fn actor_descriptor() -> DefinitionDescriptor {
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let [SchemaExtensionEntry::Definition(d)] = m.schema.as_slice() else {
        panic!("one Actor descriptor")
    };
    d.clone()
}
fn actor_program() -> RuleProgram {
    let mut p = family::program();
    let [
        RuleEffect {
            effect: RuleEffectKind::Contribute { entity, .. },
            ..
        },
    ] = p.effects.as_mut_slice()
    else {
        panic!()
    };
    assert_eq!(*entity, RuleEntity::Player);
    *entity = RuleEntity::Current;
    p
}
fn evidence(full: bool) {
    let v: Value = read("source-vectors.json");
    let old: Value = family::read("source-vectors.json");
    let pin = &v["inherited_source_vectors"];
    assert_eq!(
        pin["path"],
        "data/owned/poe2/3887ae68/player-intrinsic-life/source-vectors.json"
    );
    let bytes = fs::read(family::data().join("source-vectors.json")).unwrap();
    assert_eq!(pin["bytes"], bytes.len());
    assert_eq!(pin["sha256"], hash(&bytes));
    for field in ["reports", "initializer_branch", "compatibility_boundary"] {
        assert_eq!(v[field], old[field]);
    }
    let expected: Vec<Value> = old["native_cases"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let mut c = c.clone();
            c["source_class_id"] =
                old["projections"][i]["state"]["modes"]["MAIN"]["class_id"].clone();
            c
        })
        .collect();
    assert_eq!(v["native_cases"], json!(expected));
    assert_eq!(v["final_life_claim"], false);
    assert_eq!(v["whole_build_claim"], false);
    family::verify_source(full);
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let p: Ownership = read("ownership.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    assert_eq!(a["status"], "ready");
    assert_eq!(a["before"], b["before"]);
    assert_eq!(json!(m.before), b["before"]);
    assert_eq!(p.schema_version, 1);
    for (field, n) in [
        ("new_definitions", 1),
        ("removed_class_programs", 8),
        ("new_actor_programs", 1),
        ("new_actor_applications", 1),
        ("new_receivers", 0),
        ("closed_existing_rule_owners", 0),
    ] {
        assert_eq!(a[field], n);
    }
    assert_eq!(a["full_build_claim"], false);
    assert_eq!(a["artifacts"].as_object().unwrap().len(), 4);
    for name in [
        "bindings.json",
        "migration.json",
        "ownership.json",
        "source-vectors.json",
    ] {
        let bytes = fs::read(data().join(name)).unwrap();
        assert_eq!(a["artifacts"][name]["bytes"], bytes.len());
        assert_eq!(a["artifacts"][name]["sha256"], hash(&bytes));
        assert!(bytes.ends_with(b"\n") && !bytes.contains(&b'\r'));
    }
    let old: Value = family::read("bindings.json");
    for field in ["classes", "life", "unit"] {
        assert_eq!(b[field], old[field]);
    }
    assert_eq!(b["actor"]["key"], "def.000000000000332a");
    assert_eq!(b["registry_last_issued_before"], 0x3329);
    assert_eq!(b["registry_last_issued_after"], 0x332a);
    assert_eq!(m.schema_version, 5);
    assert_eq!(
        m.contract.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_V21
    );
    assert_eq!(m.contract.schema_version, 6);
    assert_eq!(json!(m.release), b["release"]);
    assert!(
        m.owners.is_empty()
            && m.tables.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    let descriptor = actor_descriptor();
    let actor: ActorDefId = decode(&b["actor"]);
    assert_eq!(descriptor.address(), actor.address());
    let value = json!(descriptor);
    let declarations = value["value"]["schema"]["value"]["declarations"]
        .as_object()
        .unwrap();
    assert_eq!(declarations.len(), 7);
    for d in declarations.values() {
        assert_eq!(d, &json!({"members":[],"closure":{"kind":"complete"}}));
    }
    assert_eq!(p.classes.len(), 8);
    for (row, class) in p.classes.iter().zip(b["classes"].as_array().unwrap()) {
        assert_eq!(
            row.before.owner,
            SchemaSubject::Definition(decode::<ClassDefId>(&class["class"]).address())
        );
        assert!(!row.before.programs.is_complete());
        let mut expected = row.before.clone();
        let indexes: Vec<_> = expected
            .programs
            .members
            .iter()
            .enumerate()
            .filter(|(_, v)| v.id.as_str() == "intrinsic-player-life")
            .map(|(i, _)| i)
            .collect();
        assert_eq!(indexes.len(), 1);
        assert_eq!(
            expected.programs.members.remove(indexes[0]),
            family::program()
        );
        assert_eq!(row.after, expected, "only the shared Life body moves");
    }
    assert_eq!(p.actor.owner, SchemaSubject::Definition(actor.address()));
    assert_eq!(p.actor.programs.members, vec![actor_program()]);
    assert_eq!(
        json!(p.actor.programs.closure),
        json!({"kind":"partial","value":{"gaps":[{"subject":p.actor.owner,"facet":"game_rules","code":"shared-player-initialization-not-converted"}]}})
    );
    assert_eq!(
        p.applicability,
        DeclaredSet::complete(vec![ExistingActorRuleApplication {
            id: decode(&b["application"]),
            owner: actor,
            targets: vec![ExistingActorRuleTarget::Player]
        }])
    );
    evidence(false);
}
pub fn assert_endpoint(next: &StagedOwnedRelease) {
    let p: Ownership = read("ownership.json");
    let r = &next.input().recipe.rules;
    assert_eq!(r.operations_version.as_str(), OWNED_RULE_OPERATIONS_V21);
    assert_eq!(r.existing_actor_rules, Some(p.applicability));
    let descriptor = actor_descriptor();
    assert_eq!(
        next.input()
            .recipe
            .schema
            .definitions
            .iter()
            .filter(|v| **v == descriptor)
            .count(),
        1
    );
    assert_eq!(r.owners.iter().filter(|v| **v == p.actor).count(), 1);
    for c in p.classes {
        assert_eq!(r.owners.iter().filter(|v| **v == c.after).count(), 1);
    }
    let proof = next.receipt().provenance.last().unwrap();
    let b: Value = read("bindings.json");
    assert_eq!(proof.kind.as_str(), KIND);
    assert_eq!(json!(proof.prior_input), b["before"]);
    assert_eq!(proof.authoring_input, digest());
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    evidence(true);
    let b: Value = read("bindings.json");
    for (field, actual) in [
        ("before", json!(prior.receipt().input)),
        ("definitions", json!(prior.receipt().definitions)),
        ("registry", json!(prior.receipt().registry)),
        ("rules", json!(prior.receipt().rules)),
        ("mapping", json!(prior.receipt().mapping)),
        ("roles", json!(prior.receipt().roles)),
        ("normalization", json!(prior.receipt().normalization)),
    ] {
        assert_eq!(actual, b[field]);
    }
    assert!(prior.evaluation().is_none());
    assert!(prior.input().recipe.rules.existing_actor_rules.is_none());
    let schema_step =
        compile_owned_release_migration(prior, read("migration.json"), Default::default()).unwrap();
    let p: Ownership = read("ownership.json");
    let mut input = schema_step.input().clone();
    for row in &p.classes {
        let o = input
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == row.before.owner)
            .unwrap();
        assert_eq!(*o, row.before);
        *o = row.after.clone();
    }
    assert!(
        !input
            .recipe
            .rules
            .owners
            .iter()
            .any(|o| o.owner == p.actor.owner)
    );
    input.recipe.rules.owners.push(p.actor.clone());
    input.recipe.rules.existing_actor_rules = Some(p.applicability.clone());
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: key(KIND),
        prior_input: prior.receipt().input,
        authoring_input: digest(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_endpoint(&next);
    let mut inverse = next.input().clone();
    for row in &p.classes {
        let o = inverse
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == row.after.owner)
            .unwrap();
        assert_eq!(*o, row.after);
        *o = row.before.clone();
    }
    let i = inverse
        .recipe
        .rules
        .owners
        .iter()
        .position(|o| *o == p.actor)
        .unwrap();
    inverse.recipe.rules.owners.remove(i);
    inverse.recipe.rules.existing_actor_rules = None;
    inverse.provenance = schema_step.input().provenance.clone();
    assert!(
        inverse == *schema_step.input(),
        "exact Actor ownership overlay inverse"
    );
    let mut registry =
        OwnedIdRegistry::new(prior.input().recipe.registry.clone(), Default::default()).unwrap();
    let added = registry.allocate_definition::<ActorDefinition>().unwrap();
    assert_eq!(json!(added), b["actor"]);
    assert_eq!(next.input().recipe.registry, *registry.input());
    let mut restored = inverse.recipe;
    assert_eq!(
        restored.schema.definitions.len(),
        prior.input().recipe.schema.definitions.len() + 1
    );
    restored
        .schema
        .definitions
        .retain(|d| d.address() != added.address());
    restored.registry = prior.input().recipe.registry.clone();
    restored.schema.release = prior.input().recipe.schema.release.clone();
    restored.rules.release = prior.input().recipe.rules.release.clone();
    restored.rules.definitions = prior.input().recipe.rules.definitions.clone();
    restored.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert!(
        restored == prior.input().recipe,
        "exact predecessor recipe inverse including ordered queries and all unrelated owners"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}

#[cfg(test)]
mod native {
    use super::*;
    use crate::{empty_support, reference_native, release};
    use poe_optimizer_core::owned_build::*;
    use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
    use poe_optimizer_engine::owned_plan::*;
    use poe_optimizer_import::owned_recipe::OwnedRecipeInput;
    use rayon::prelude::*;
    use std::sync::OnceLock;

    fn def<K: DefinitionDomain>(n: u64) -> DefId<K> {
        DefId::parse(
            GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap(),
            format!("def.{n:016x}"),
        )
        .unwrap()
    }
    struct Source {
        recipe: OwnedRecipeInput,
        selected: Vec<CharacterSpec>,
        vectors: Value,
        bindings: Value,
        ownership: Ownership,
    }
    fn source() -> &'static Source {
        static SOURCE: OnceLock<Source> = OnceLock::new();
        SOURCE.get_or_init(|| {
            let p = PathBuf::from(
                std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_RULE_OWNERSHIP_RELEASE")
                    .expect("verified shared Player release"),
            );
            let inventory = release::inventory(&p);
            let endpoint = release::load(&p);
            assert_endpoint(&endpoint);
            let load = |name: &str| -> Value {
                serde_json::from_slice(&fs::read(p.parent().unwrap().join(name)).unwrap()).unwrap()
            };
            let selected = (1..=5)
                .map(|i| {
                    let draft = load(&format!("original-{i:02}/draft.json"));
                    let request = load(&format!("selected-{i:02}.json"));
                    let rows = draft["draft"]["character_presets"]["members"]
                        .as_array()
                        .unwrap();
                    let row = rows
                        .iter()
                        .find(|r| r["id"] == request["build"]["character"])
                        .unwrap();
                    assert_eq!(row["class"]["kind"], "known");
                    assert_eq!(row["level"]["kind"], "known");
                    CharacterSpec {
                        class: decode(&row["class"]["value"]),
                        level: decode(&row["level"]["value"]),
                        ascendancy: None,
                        rewards: vec![],
                    }
                })
                .collect();
            assert_eq!(inventory, release::inventory(&p));
            Source {
                recipe: endpoint.input().recipe.clone(),
                selected,
                vectors: read("source-vectors.json"),
                bindings: read("bindings.json"),
                ownership: read("ownership.json"),
            }
        })
    }
    struct World {
        recipe: OwnedRecipeInput,
        build: BuildInput,
        scenario: ScenarioInput,
    }
    impl World {
        fn new(class: ClassDefId, level: u16) -> Self {
            // The reused fixture excludes items, passives, skills and external
            // effects. Its complete inventories are intrinsic-component test
            // authority only, never a published game completeness statement.
            let (mut recipe, build, scenario) = reference_native::finite_parts(class, level);
            let s = source();
            for row in &s.ownership.classes {
                let owner = recipe
                    .rules
                    .owners
                    .iter_mut()
                    .find(|o| o.owner == row.before.owner)
                    .unwrap();
                assert_eq!(
                    owner.programs,
                    DeclaredSet::complete(vec![family::program()])
                );
                owner.programs.members.clear();
            }
            recipe.registry = s.recipe.registry.clone();
            recipe.schema.release = s.recipe.schema.release.clone();
            recipe.schema.definitions.push(actor_descriptor());
            let mut actor = s.ownership.actor.clone();
            actor.programs = DeclaredSet::complete(actor.programs.members);
            recipe.rules.owners.push(actor);
            recipe.rules.operations_version = s.recipe.rules.operations_version.clone();
            // This finite component contains no ordered reduction consumer.
            // Production's Partial attribute memberships remain unchanged.
            recipe.rules.ordered_contributions = Some(DeclaredSet::complete(vec![]));
            recipe.rules.existing_actor_rules = s.recipe.rules.existing_actor_rules.clone();
            Self {
                recipe,
                build,
                scenario,
            }
        }
        fn plan(
            &self,
        ) -> std::result::Result<OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>, PlanError>
        {
            empty_support::compile(&self.recipe, &self.build, &self.scenario, def(2))
        }
        fn report(&self) -> SupportEffectsReport {
            let p = self.plan().unwrap();
            p.evaluate(&mut p.new_scratch()).unwrap()
        }
        fn effects(&self) -> OwnedEffectsReport {
            effects(self.report())
        }
        fn assert_value(&self, r: &OwnedEffectsReport, value: f64) {
            assert!(r.gaps.is_empty(), "{:?}", r.gaps);
            assert_eq!(r.effects.len(), 1, "one Player application, no Class copy");
            let e = &r.effects[0];
            assert_eq!(e.key.invocation.owner, source().ownership.actor.owner);
            assert_eq!(e.key.invocation.program.as_str(), "intrinsic-player-life");
            assert_eq!(
                e.key.invocation.entity,
                ConcreteEntity::Actor(ActorKey::Player)
            );
            assert_eq!(
                e.key.invocation.origin,
                RuleOrigin::ExistingActor {
                    application: key("shared-player-initialization"),
                    actor: ActorKey::Player
                }
            );
            assert_eq!(
                e.target,
                BoundEffectTarget::Contribution {
                    key: ContributionKey {
                        entity: ConcreteEntity::Actor(ActorKey::Player),
                        stat: def(0x311a),
                        kind: ContributionKind::Add
                    }
                }
            );
            assert_eq!(
                e.value,
                EffectValue::Known {
                    value: ParameterValue::Quantity(
                        FiniteQuantity::new(value, def(0x3119)).unwrap()
                    )
                }
            );
            assert!(
                !r.values
                    .iter()
                    .any(|v| matches!(&v.key,PlanValueKey::Stat {stat,..} if *stat==def(0x311a))),
                "intrinsic Life is not final Life"
            );
        }
    }
    fn effects(r: SupportEffectsReport) -> OwnedEffectsReport {
        let SupportEffectsOutcome::Evaluated { effects } = r.outcome else {
            panic!("{r:?}")
        };
        assert_eq!(r.gaps, effects.gaps);
        effects
    }
    fn source_value(level: u16) -> f64 {
        source().vectors["native_cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["character_level"] == level)
            .unwrap()["intrinsic_life"]
            .as_f64()
            .unwrap()
    }
    #[test]
    #[ignore = "requires PLAYER_RULE_OWNERSHIP_RELEASE and PLAYER_INTRINSIC_LIFE_RELEASE"]
    fn actual_five_inputs_and_source_controls_deliver_one_existing_player_contribution() {
        let s = source();
        let cases = s.vectors["native_cases"].as_array().unwrap();
        assert_eq!(cases.len(), 10);
        for (i, c) in cases.iter().enumerate() {
            let binding = s.bindings["classes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["source_id"] == c["source_class_id"])
                .unwrap();
            let class: ClassDefId = decode(&binding["class"]);
            let level: u16 = decode(&c["character_level"]);
            if i < 5 {
                assert_eq!(s.selected[i].class, class);
                assert_eq!(s.selected[i].level, level);
            }
            let w = World::new(class.clone(), level);
            let actual = w.effects();
            w.assert_value(&actual, c["intrinsic_life"].as_f64().unwrap());
            // Same exact old body, numerical target/value preserved across the
            // ownership change; old Provider origin is intentionally different.
            let (old, build, scenario) = reference_native::finite_parts(class, level);
            let p = empty_support::compile(&old, &build, &scenario, def(2)).unwrap();
            let old = effects(p.evaluate(&mut p.new_scratch()).unwrap());
            assert_eq!(old.effects.len(), 1);
            assert_eq!(actual.effects[0].target, old.effects[0].target);
            assert_eq!(actual.effects[0].value, old.effects[0].value);
            assert_ne!(
                actual.effects[0].key.invocation.origin,
                old.effects[0].key.invocation.origin
            );
        }
    }
    #[test]
    #[ignore = "requires PLAYER_RULE_OWNERSHIP_RELEASE and PLAYER_INTRINSIC_LIFE_RELEASE"]
    fn all_eight_classes_share_the_same_single_initializer_at_source_level_boundaries() {
        for c in source().bindings["classes"].as_array().unwrap() {
            for level in [1, 92, 100] {
                let w = World::new(decode(&c["class"]), level);
                w.assert_value(&w.effects(), source_value(level));
            }
        }
        for level in [0, 101] {
            let w = World::new(source().selected[4].class.clone(), level);
            assert!(
                matches!(w.plan(),Err(PlanError::Invalid(ref message)) if message=="owned request has invalid schema bindings")
            );
        }
    }
    #[test]
    #[ignore = "requires PLAYER_RULE_OWNERSHIP_RELEASE and PLAYER_INTRINSIC_LIFE_RELEASE"]
    fn actual_partial_actor_and_class_coverage_refuse_and_missing_application_never_defaults() {
        let s = source();
        let class = s.selected[4].class.clone();
        let class_owner = s
            .ownership
            .classes
            .iter()
            .find(|o| o.after.owner == SchemaSubject::Definition(class.address()))
            .unwrap();
        for actual in [&s.ownership.actor, &class_owner.after] {
            let mut w = World::new(class.clone(), 92);
            // Restore the exact production closure while retaining this finite
            // component's bodies. This tests coverage, not other Class mechanics.
            let o = w
                .recipe
                .rules
                .owners
                .iter_mut()
                .find(|o| o.owner == actual.owner)
                .unwrap();
            o.programs.closure = actual.programs.closure.clone();
            let report = w.report();
            assert!(
                report
                    .gaps
                    .iter()
                    .any(|g| g.reason == PlanGapReason::PartialPrograms
                        && g.subject.as_ref() == Some(&actual.owner))
            );
            assert_eq!(
                report.outcome,
                SupportEffectsOutcome::Unavailable {
                    cause: EffectValue::Unresolved {
                        reason: PlanGapReason::IncompleteContributors,
                        read: None
                    },
                    input: None
                }
            );
        }
        for remove_body in [false, true] {
            let mut w = World::new(class.clone(), 92);
            if remove_body {
                w.recipe
                    .rules
                    .owners
                    .iter_mut()
                    .find(|o| o.owner == s.ownership.actor.owner)
                    .unwrap()
                    .programs
                    .members
                    .clear();
            } else {
                w.recipe.rules.existing_actor_rules = Some(DeclaredSet::complete(vec![]));
            }
            let result = w.effects();
            assert!(result.effects.is_empty());
            assert!(
                !result
                    .values
                    .iter()
                    .any(|v| matches!(&v.key,PlanValueKey::Stat {stat,..} if *stat==def(0x311a)))
            );
        }
    }
    #[test]
    #[ignore = "requires PLAYER_RULE_OWNERSHIP_RELEASE and PLAYER_INTRINSIC_LIFE_RELEASE"]
    fn fresh_reused_and_rayon_actor_applications_are_deterministic() {
        let a = World::new(source().selected[4].class.clone(), 92);
        let b = World::new(source().selected[1].class.clone(), 91);
        let pa = a.plan().unwrap();
        let pb = b.plan().unwrap();
        let va = a.effects();
        let vb = b.effects();
        a.assert_value(&va, source_value(92));
        b.assert_value(&vb, source_value(91));
        assert_ne!(pa.identity(), pb.identity());
        let mut scratch = pa.new_scratch();
        assert_eq!(effects(pa.evaluate(&mut scratch).unwrap()), va);
        assert_eq!(effects(pb.evaluate(&mut scratch).unwrap()), vb);
        assert_eq!(effects(pa.evaluate(&mut scratch).unwrap()), va);
        let expected: Vec<_> = (0..16)
            .map(|i| if i % 2 == 0 { va.clone() } else { vb.clone() })
            .collect();
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .unwrap();
        let actual: Vec<_> = pool.install(|| {
            (0..16)
                .into_par_iter()
                .map(|i| {
                    let p = if i % 2 == 0 { &pa } else { &pb };
                    effects(p.evaluate(&mut p.new_scratch()).unwrap())
                })
                .collect()
        });
        assert_eq!(actual, expected);
    }
}
