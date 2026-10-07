//! Authored scalar consumers over real Count streams. Production membership and
//! effective-MORE composition remain explicitly unproved; no fallback is added.
use super::{empty_support, migration_preservation, release};
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_import::{
    owned_mapping::OwnedIdRegistry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};
pub const KIND: &str = "attribute-step-consumers-v1";
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/attribute-step-consumers")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    s.parse().unwrap()
}
fn decode<T: DeserializeOwned>(v: &Value) -> T {
    serde_json::from_value(v.clone()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn digest() -> OwnedContentDigest {
    let values: Vec<Value> = [
        "authoring.json",
        "bindings.json",
        "schema-migration.json",
        "consumers.json",
        "queries.json",
        "source-vectors.json",
    ]
    .map(read)
    .into();
    digest_owned(KIND, &values, 1024 * 1024).unwrap()
}
#[derive(Clone, Deserialize)]
pub struct Consumers {
    pub schema_version: u32,
    pub owners: Vec<DefinitionRules>,
    pub receivers: Vec<StatReceiver>,
}
fn rows(v: &Value) -> &[Value] {
    if let Some(a) = v.as_array() {
        a
    } else {
        assert!(v.as_object().is_some_and(|o| o.is_empty()));
        &[]
    }
}
pub fn project_case(c: &Value) -> Value {
    let mut modes = serde_json::Map::new();
    for mode in ["MAIN", "CALCS"] {
        let m = &c["original"]["state"]["modes"][mode];
        let stages: Vec<_> = rows(&m["provenance"]["stages"]).iter().map(|s| {
            assert_eq!(rows(&s["before_chain"]).len(), 1);
            assert!(rows(&s["reads"]).is_empty());
            let chain = &s["before_chain"][0];
            let records: Vec<_> = rows(&chain["attributes"][s["stat"].as_str().unwrap()]).iter().map(|r| {
                let v = &r["record"];
                assert_eq!(v["value"]["root"]["kind"], "number");
                json!({"source":v["source"],"name":v["name"],"type":v["type"],
                    "flags":v["flags"],"keyword_flags":v["keyword_flags"],"tag_count":v["tag_count"],
                    "index":r["index"],"value":v["value"]["root"]["value"]})
            }).collect();
            json!({"index":s["index"],"pass":s["pass"],"stat":s["stat"],"value":s["value"],
                "records":records,"queries":s["queries"],"reads":s["reads"],"depth":chain["depth"],"parent_kind":chain["parent_kind"]})
        }).collect();
        modes.insert(
            mode.into(),
            json!({"class_id":m["class_id"],"stages":stages}),
        );
    }
    json!({"name":c["name"],"xml_sha256":c["xml_sha256"],"choice_control":c["choice_control"],"modes":modes})
}
pub fn source(full: bool) {
    let v: Value = read("source-vectors.json");
    assert_eq!(v["schema_version"], 1);
    assert_eq!(rows(&v["cases"]).len(), 3);
    for (i, c) in rows(&v["cases"]).iter().enumerate() {
        assert_eq!(
            c["name"],
            [
                "original-05",
                "original-05-strength-choice-to-dexterity",
                "restored-original-05"
            ][i]
        );
        for mode in ["MAIN", "CALCS"] {
            let stages = rows(&c["modes"][mode]["stages"]);
            assert_eq!(stages.len(), 6);
            for (s, stage) in stages.iter().enumerate() {
                assert_eq!(stage["index"], s + 1);
                assert_eq!(stage["pass"], s / 3 + 1);
                assert_eq!(stage["stat"], ["Str", "Dex", "Int"][s % 3]);
                assert_eq!(stage["depth"], 0);
                assert!(rows(&stage["reads"]).is_empty());
                assert_eq!(
                    stage["value"],
                    if i == 1 {
                        [22, 12, 105][s % 3]
                    } else {
                        [27, 7, 105][s % 3]
                    }
                );
                let queries = rows(&stage["queries"]);
                assert_eq!(queries.len(), 3);
                for (q, kind) in queries.iter().zip(["BASE", "INC", "MORE"]) {
                    assert_eq!(q["contribution"], kind);
                    assert_eq!(q["actual_return_local"], true);
                    assert_eq!(q["original_context_is_player"], true);
                    assert_eq!(q["store_depth"], 0);
                    assert_eq!(q["name"], stage["stat"]);
                }
                assert_eq!(queries[1]["result"], 0);
                assert_eq!(queries[2]["result"], 1);
                for (index, r) in rows(&stage["records"]).iter().enumerate() {
                    assert_eq!(r["index"], index + 1);
                    assert_eq!(r["type"], "BASE");
                    assert_eq!(r["name"], stage["stat"]);
                    assert_eq!(r["flags"], 0);
                    assert_eq!(r["keyword_flags"], 0);
                    assert_eq!(r["tag_count"], 0);
                    assert!(r["value"].as_u64().is_some_and(|n| (1..=15).contains(&n)));
                }
            }
        }
    }
    if !full {
        return;
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for (name, pin) in v["source_pins"].as_object().unwrap() {
        let bytes = fs::read(root.join(name)).unwrap();
        assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().unwrap());
        assert_eq!(hash(&bytes), pin["sha256"]);
    }
    let mut prior = None;
    for pin in rows(&v["source_reports"]) {
        let bytes = fs::read(root.join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().unwrap());
        let digest = hash(&bytes);
        assert_eq!(digest, pin["sha256"]);
        if let Some(old) = prior.replace(digest) {
            assert_eq!(Some(&old), prior.as_ref());
        }
        let report: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            report["observer_sha256"],
            v["source_pins"]["crates/poe-optimizer-pob/tests/support/attribute_pipeline_source.lua"]
                ["sha256"]
        );
        assert_eq!(
            report["source_revision"],
            "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
        );
        let projected: Vec<_> = [4, 5, 6].map(|i| project_case(&report["cases"][i])).into();
        assert_eq!(json!(projected), v["cases"]);
    }
}
fn expected_program(stage: &Value, b: &Value) -> RuleProgram {
    let count: UnitDefId = decode(&b["count_unit"]);
    let factor: UnitDefId = decode(&b["factor_unit"]);
    let percent: UnitDefId = decode(&b["percent_unit"]);
    let quantity = |n, unit: &UnitDefId| {
        ParameterValue::Quantity(FiniteQuantity::new(n, unit.clone()).unwrap())
    };
    let literal = |id: &str, value| RuleNode {
        id: key(id),
        expression: RuleExpression::Literal { value },
    };
    let node = |id: &str, expression| RuleNode {
        id: key(id),
        expression,
    };
    let qtype = |unit| ComputedValueType::Quantity { unit };
    let mut nodes: Vec<_> = ["base", "increase", "effective-more"]
        .map(|id| node(id, RuleExpression::Read { input: key(id) }))
        .into();
    nodes.extend([
        literal("zero-count", quantity(0., &count)),
        literal("half-count", quantity(0.5, &count)),
        literal("one-factor", quantity(1., &factor)),
        literal(
            "zero-integer",
            ParameterValue::Integer(BoundedInteger::new(0).unwrap()),
        ),
        node(
            "zero-base",
            RuleExpression::Compare {
                operation: RuleComparison::Equal,
                left: key("base"),
                right: key("zero-count"),
            },
        ),
        node(
            "increase-fraction",
            RuleExpression::PercentAsFactor {
                percent: key("increase"),
                unit: factor.clone(),
            },
        ),
        node(
            "increased-factor",
            RuleExpression::Add {
                left: key("one-factor"),
                right: key("increase-fraction"),
            },
        ),
        node(
            "combined-factor",
            RuleExpression::Scale {
                value: key("increased-factor"),
                factor: key("effective-more"),
            },
        ),
        node(
            "scaled",
            RuleExpression::Scale {
                value: key("base"),
                factor: key("combined-factor"),
            },
        ),
        node(
            "plus-half",
            RuleExpression::Add {
                left: key("scaled"),
                right: key("half-count"),
            },
        ),
        node(
            "floored",
            RuleExpression::Round {
                value: key("plus-half"),
                quantum: FiniteQuantity::new(1., count.clone()).unwrap(),
                mode: RuleRounding::Floor,
            },
        ),
        node(
            "clamped",
            RuleExpression::Maximum {
                left: key("floored"),
                right: key("zero-count"),
            },
        ),
        node(
            "integer",
            RuleExpression::QuantizeInteger {
                value: key("clamped"),
                quantum: FiniteQuantity::new(1., count.clone()).unwrap(),
                mode: RuleRounding::Floor,
            },
        ),
        node(
            "result",
            RuleExpression::Select {
                condition: key("zero-base"),
                when_true: key("zero-integer"),
                when_false: key("integer"),
            },
        ),
    ]);
    RuleProgram {
        id: decode(&stage["program"]),
        context: RuleEntityKind::Actor,
        reads: vec![
            RuleRead {
                id: key("base"),
                value_type: qtype(count),
                source: RuleReadSource::ContributionQuery {
                    entity: RuleEntity::Current,
                    query: decode(&stage["base_query"]),
                    group: decode(&stage["group"]),
                },
            },
            RuleRead {
                id: key("increase"),
                value_type: qtype(percent),
                source: RuleReadSource::ContributionQuery {
                    entity: RuleEntity::Current,
                    query: decode(&stage["increase_query"]),
                    group: decode(&stage["group"]),
                },
            },
            RuleRead {
                id: key("effective-more"),
                value_type: qtype(factor),
                source: RuleReadSource::Stat {
                    entity: RuleEntity::Current,
                    stat: decode(&stage["effective_more"]),
                },
            },
        ],
        nodes,
        effects: vec![RuleEffect {
            id: key("result"),
            when: None,
            effect: RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat: decode(&stage["output"]),
                value: key("result"),
            },
        }],
    }
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let c: Consumers = read("consumers.json");
    let m: OwnedReleaseMigrationInput = read("schema-migration.json");
    let q: DeclaredSet<ContributionQuery> = read("queries.json");
    assert_eq!(a["schema_version"], 1);
    assert_eq!(c.schema_version, 1);
    assert_eq!(a["before"], b["before"]);
    assert_eq!(json!(m.before), b["before"]);
    for (name, pin) in a["artifacts"].as_object().unwrap() {
        let bytes = fs::read(data().join(name)).unwrap();
        assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().unwrap());
        assert_eq!(hash(&bytes), pin["sha256"]);
        assert!(bytes.ends_with(b"\n") && !bytes.contains(&b'\r'));
    }
    assert_eq!(a["artifacts"].as_object().unwrap().len(), 5);
    assert_eq!(m.schema_version, 5);
    assert_eq!(
        m.contract.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_V20
    );
    assert_eq!(m.schema.len(), 9);
    assert!(
        m.owners.is_empty()
            && m.receivers.is_empty()
            && m.tables.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    for (i, s) in m.schema.iter().enumerate() {
        let v = json!(s);
        assert_eq!(
            v["value"]["value"]["id"]["key"],
            format!("def.{:016x}", 0x3321 + i)
        );
        let schema = &v["value"]["value"]["schema"]["value"];
        assert_eq!(schema["targets"], json!(["actor"]));
        assert_eq!(
            schema["value"],
            if i < 3 {
                json!({"kind":"integer"})
            } else {
                json!({"kind":"quantity","value":{"unit":b["factor_unit"]}})
            }
        );
    }
    assert_eq!(c.owners.len(), 12);
    assert_eq!(c.receivers.len(), 6);
    assert_eq!(q.members.len(), 12);
    assert!(!q.is_complete());
    let mut ids = BTreeSet::new();
    for (i, s) in rows(&b["stages"]).iter().enumerate() {
        assert_eq!(s["input"]["key"], format!("def.{:016x}", 0x331b + i));
        assert_eq!(
            s["output"]["key"],
            format!(
                "def.{:016x}",
                if i < 3 { 0x3321 + i } else { 0x1d2e + i - 3 }
            )
        );
        assert_eq!(
            s["effective_more"]["key"],
            format!("def.{:016x}", 0x3324 + i)
        );
        let output: StatDefId = decode(&s["output"]);
        let factor: StatDefId = decode(&s["effective_more"]);
        let owner = c
            .owners
            .iter()
            .find(|o| o.owner == SchemaSubject::Definition(output.address()))
            .unwrap();
        assert_eq!(
            owner.programs,
            DeclaredSet::complete(vec![expected_program(s, &b)])
        );
        let unresolved = c
            .owners
            .iter()
            .find(|o| o.owner == SchemaSubject::Definition(factor.address()))
            .unwrap();
        assert!(unresolved.programs.members.is_empty() && !unresolved.programs.is_complete());
        assert_eq!(
            c.receivers[i],
            StatReceiver {
                id: decode(&s["receiver"]),
                stat: output,
                program: decode(&s["program"]),
                targets: vec![StatReceiverTarget::Player]
            }
        );
        for (field, kind, unit) in [
            ("base_query", ContributionKind::Add, &b["count_unit"]),
            (
                "increase_query",
                ContributionKind::Increase,
                &b["percent_unit"],
            ),
        ] {
            let row = q.members.iter().find(|q| json!(q.id) == s[field]).unwrap();
            assert!(ids.insert(row.id.clone()));
            assert_eq!(json!(row.stat), s["input"]);
            assert_eq!(row.contribution, kind);
            assert_eq!(row.groups.len(), 1);
            let g = &row.groups[0];
            assert_eq!(json!(g.id), s["group"]);
            assert_eq!(g.reduction, ContributionReduction::Sum);
            assert_eq!(
                g.empty,
                ParameterValue::Quantity(FiniteQuantity::new(0., decode(unit)).unwrap())
            );
            assert!(
                !g.members.is_complete() && g.members.members.is_empty(),
                "no production ranks or complete empty defaults"
            );
        }
    }
    assert_eq!(ids.len(), 12);
    source(false);
}
pub fn assert_endpoint(next: &StagedOwnedRelease) {
    let c: Consumers = read("consumers.json");
    let q: DeclaredSet<ContributionQuery> = read("queries.json");
    assert_eq!(
        next.input().recipe.rules.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_V21
    );
    assert_eq!(next.input().recipe.rules.contribution_queries, Some(q));
    for o in c.owners {
        assert_eq!(
            next.input()
                .recipe
                .rules
                .owners
                .iter()
                .filter(|v| **v == o)
                .count(),
            1
        );
    }
    for r in c.receivers {
        assert_eq!(
            next.input()
                .recipe
                .rules
                .receivers
                .members
                .iter()
                .filter(|v| **v == r)
                .count(),
            1
        );
    }
    let proof = next.receipt().provenance.last().unwrap();
    let b: Value = read("bindings.json");
    assert_eq!(proof.kind.as_str(), KIND);
    assert_eq!(json!(proof.prior_input), b["before"]);
    assert_eq!(proof.authoring_input, digest());
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    source(true);
    let b: Value = read("bindings.json");
    let m: OwnedReleaseMigrationInput = read("schema-migration.json");
    for (field, actual) in [
        ("before", json!(prior.receipt().input)),
        ("definitions", json!(prior.receipt().definitions)),
        ("registry", json!(prior.receipt().registry)),
        ("rules", json!(prior.receipt().rules)),
    ] {
        assert_eq!(actual, b[field]);
    }
    assert!(prior.evaluation().is_none());
    assert!(prior.input().recipe.rules.contribution_queries.is_none());
    let schema_step =
        compile_owned_release_migration(prior, m.clone(), Default::default()).unwrap();
    let mut input = schema_step.input().clone();
    let c: Consumers = read("consumers.json");
    for o in &c.owners {
        assert!(!input.recipe.rules.owners.iter().any(|v| v.owner == o.owner));
    }
    for r in &c.receivers {
        assert!(
            !input
                .recipe
                .rules
                .receivers
                .members
                .iter()
                .any(|v| v.id == r.id || v.stat == r.stat)
        );
    }
    input.recipe.rules.owners.extend(c.owners.clone());
    input
        .recipe
        .rules
        .receivers
        .members
        .extend(c.receivers.clone());
    input.recipe.rules.operations_version = key(OWNED_RULE_OPERATIONS_V21);
    input.recipe.rules.contribution_queries = Some(read("queries.json"));
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: key(KIND),
        prior_input: prior.receipt().input,
        authoring_input: digest(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_endpoint(&next);
    let mut inverse = next.input().clone();
    for o in &c.owners {
        let index = inverse
            .recipe
            .rules
            .owners
            .iter()
            .position(|v| v == o)
            .unwrap();
        inverse.recipe.rules.owners.remove(index);
    }
    for r in &c.receivers {
        let index = inverse
            .recipe
            .rules
            .receivers
            .members
            .iter()
            .position(|v| v == r)
            .unwrap();
        inverse.recipe.rules.receivers.members.remove(index);
    }
    inverse.recipe.rules.operations_version =
        schema_step.input().recipe.rules.operations_version.clone();
    inverse.recipe.rules.contribution_queries = None;
    inverse.provenance = schema_step.input().provenance.clone();
    assert!(
        inverse == *schema_step.input(),
        "exact V21 overlay inverse differs from checked schema-only stage"
    );
    let mut registry =
        OwnedIdRegistry::new(prior.input().recipe.registry.clone(), Default::default()).unwrap();
    let added: Vec<_> = (0..9)
        .map(|_| {
            registry
                .allocate_definition::<StatDefinition>()
                .unwrap()
                .address()
        })
        .collect();
    assert_eq!(next.input().recipe.registry, *registry.input());
    let mut restored = inverse.recipe;
    assert_eq!(
        restored.schema.definitions.len(),
        prior.input().recipe.schema.definitions.len() + 9
    );
    restored
        .schema
        .definitions
        .retain(|d| !added.contains(&d.address()));
    restored.registry = prior.input().recipe.registry.clone();
    restored.schema.release = prior.input().recipe.schema.release.clone();
    restored.rules.release = prior.input().recipe.rules.release.clone();
    restored.rules.definitions = prior.input().recipe.rules.definitions.clone();
    restored.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert!(
        restored == prior.input().recipe,
        "exact predecessor recipe inverse"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}

/// Shared finite class/choice fixture. Its complete inventories and source-bound
/// member ranks are test authority only; callers must preserve that distinction.
pub(super) fn finite_parts() -> (
    poe_optimizer_import::owned_recipe::OwnedRecipeInput,
    poe_optimizer_core::owned_build::BuildInput,
) {
    native::finite_parts()
}

#[cfg(test)]
mod native {
    use super::*;
    use poe_optimizer_core::owned_build::*;
    use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
    use poe_optimizer_engine::{
        owned_plan::*,
        owned_rules::{CompiledRulePackage, EffectDisposition, NumericalFailure, RuleFact},
    };
    use poe_optimizer_import::owned_recipe::OwnedRecipeInput;
    use rayon::prelude::*;
    use std::sync::OnceLock;

    fn ns() -> GameVersionNamespace {
        GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap()
    }
    fn def<K: DefinitionDomain>(n: u64) -> DefId<K> {
        DefId::parse(ns(), format!("def.{n:016x}")).unwrap()
    }
    fn subject(id: StatDefId) -> SchemaSubject {
        SchemaSubject::Definition(id.address())
    }
    struct Source {
        recipe: OwnedRecipeInput,
        tree: Value,
        vectors: Value,
    }
    fn source() -> &'static Source {
        static SOURCE: OnceLock<Source> = OnceLock::new();
        SOURCE.get_or_init(|| {
            let path = PathBuf::from(
                std::env::var_os("POE_OPTIMIZER_TEST_ATTRIBUTE_STEP_RELEASE")
                    .expect("verified attribute consumer release"),
            );
            let before = release::inventory(&path);
            let endpoint = release::load(&path);
            assert_endpoint(&endpoint);
            let source = Source {
                recipe: endpoint.input().recipe.clone(),
                tree: json!(endpoint.input().tree),
                vectors: read("source-vectors.json"),
            };
            assert_eq!(before, release::inventory(&path));
            source
        })
    }
    #[derive(Clone)]
    struct World {
        recipe: OwnedRecipeInput,
        build: BuildInput,
    }
    pub(super) fn finite_parts() -> (OwnedRecipeInput, BuildInput) {
        let world = World::assemble();
        (world.recipe, world.build)
    }
    impl World {
        fn new() -> Self {
            let (recipe, build) = super::finite_parts();
            Self { recipe, build }
        }
        fn assemble() -> Self {
            let (mut recipe, build) = crate::count_native::finite_parts(4);
            let source = source();
            let consumers: Consumers = read("consumers.json");
            recipe.registry = source.recipe.registry.clone();
            recipe.schema.release = source.recipe.schema.release.clone();
            for n in 0x3321..=0x3329 {
                let descriptor = source
                    .recipe
                    .schema
                    .definitions
                    .iter()
                    .find(|d| d.address() == def::<StatDefinition>(n).address())
                    .unwrap();
                assert!(
                    !recipe
                        .schema
                        .definitions
                        .iter()
                        .any(|d| d.address() == descriptor.address())
                );
                recipe.schema.definitions.push(descriptor.clone());
            }
            recipe.rules.operations_version = key(OWNED_RULE_OPERATIONS_V21);
            recipe.rules.contribution_queries = source.recipe.rules.contribution_queries.clone();
            for owner in consumers.owners {
                if let Some(old) = recipe
                    .rules
                    .owners
                    .iter_mut()
                    .find(|o| o.owner == owner.owner)
                {
                    assert!(old.programs.members.is_empty());
                    *old = owner;
                } else {
                    recipe.rules.owners.push(owner);
                }
            }
            recipe.rules.receivers = DeclaredSet::complete(consumers.receivers);
            let mut world = Self { recipe, build };
            world.finite_members();
            world.finite_factors();
            world
        }
        fn source_name(&self, owner: &SchemaSubject) -> String {
            match owner {
                SchemaSubject::Definition(DefinitionAddress::Class(id)) => {
                    assert_eq!(id, &self.build.character.class);
                    "Base".into()
                }
                SchemaSubject::Definition(DefinitionAddress::PassiveNode(node)) => {
                    assert!(self.build.allocations.iter().any(|a| &a.node == node));
                    let matches: Vec<_> = rows(&source().tree["content"]["tokens"])
                        .iter()
                        .filter(|r| {
                            r["role"]["kind"] == "allocation"
                                && r["role"]["value"]["node"] == json!(node)
                        })
                        .collect();
                    assert_eq!(matches.len(), 1);
                    format!("Tree:{}", matches[0]["token"].as_str().unwrap())
                }
                _ => panic!("finite class/choice origin only"),
            }
        }
        fn finite_members(&mut self) {
            let mut registry = self.recipe.rules.contribution_queries.take().unwrap();
            assert!(!registry.is_complete());
            registry.closure = SchemaClosure::Complete;
            for (index, query) in registry.members.iter_mut().enumerate() {
                let stage = index / 2;
                let mut members = vec![];
                let source_rows = rows(
                    &source().vectors["cases"][0]["modes"]["MAIN"]["stages"][stage]["records"],
                );
                let changed_rows = rows(
                    &source().vectors["cases"][1]["modes"]["MAIN"]["stages"][stage]["records"],
                );
                let mut inactive_rank = 1000;
                for owner in &self.recipe.rules.owners {
                    let selected = match &owner.owner {
                        SchemaSubject::Definition(DefinitionAddress::Class(id)) => {
                            id == &self.build.character.class
                        }
                        SchemaSubject::Definition(DefinitionAddress::PassiveNode(id)) => {
                            self.build.allocations.iter().any(|a| &a.node == id)
                        }
                        _ => false,
                    };
                    if !selected {
                        continue;
                    }
                    for program in &owner.programs.members {
                        for (effect_rank, effect) in program.effects.iter().enumerate() {
                            let RuleEffectKind::Contribute {
                                stat, contribution, ..
                            } = &effect.effect
                            else {
                                continue;
                            };
                            if stat != &query.stat || contribution != &query.contribution {
                                continue;
                            }
                            let name = self.source_name(&owner.owner);
                            // Only executed records have source-order authority. Other
                            // potential choice effects are inactive in both controls;
                            // their explicit fixture ranks make no source-order claim.
                            let observed = source_rows
                                .iter()
                                .chain(changed_rows)
                                .find(|r| r["source"] == name);
                            let rank = if let Some(r) = observed {
                                r["index"].as_u64().unwrap() as u32
                            } else {
                                inactive_rank += 1;
                                inactive_rank
                            };
                            let origin = if name == "Base" {
                                ContributionOrigin::Character
                            } else {
                                ContributionOrigin::Allocation
                            };
                            members.push(ContributionMember {
                                owner: owner.owner.clone(),
                                program: program.id.clone(),
                                effect: effect.id.clone(),
                                origin,
                                order: Some(ContributionOrder {
                                    source_rank: rank,
                                    program_rank: 0,
                                    effect_rank: effect_rank as u32,
                                    slot_ranks: vec![],
                                }),
                            });
                        }
                    }
                }
                let group = &mut query.groups[0];
                assert!(group.members.members.is_empty() && !group.members.is_complete());
                if query.contribution == ContributionKind::Add {
                    assert_eq!(members.len(), 23);
                } else {
                    assert!(members.is_empty());
                }
                group.members = DeclaredSet::complete(members);
            }
            self.recipe.rules.contribution_queries = Some(registry);
        }
        fn finite_factors(&mut self) {
            // The finite fixture admits exactly one class and 22 choice sources.
            // Source05 proves no MORE records in this selected slice. The empty
            // Product identity belongs to this fixture only, independent of any
            // nonempty grouping/rounding law. Production has no such producer.
            for i in 0..6 {
                let output = def(0x3324 + i);
                let owner = self
                    .recipe
                    .rules
                    .owners
                    .iter_mut()
                    .find(|o| o.owner == subject(output.clone()))
                    .unwrap();
                assert!(!owner.programs.is_complete() && owner.programs.members.is_empty());
                let program = key(&format!("fixture-empty-more-{i}"));
                owner.programs = DeclaredSet::complete(vec![RuleProgram {
                    id: program.clone(),
                    context: RuleEntityKind::Actor,
                    reads: vec![RuleRead {
                        id: key("incoming"),
                        value_type: ComputedValueType::Quantity { unit: def(1) },
                        source: RuleReadSource::Contributions {
                            entity: RuleEntity::Current,
                            stat: def(0x331b + i),
                            contribution: ContributionKind::Multiply,
                            reduction: ContributionReduction::Product,
                            empty: ParameterValue::Quantity(
                                FiniteQuantity::new(1., def(1)).unwrap(),
                            ),
                        },
                    }],
                    nodes: vec![RuleNode {
                        id: key("factor"),
                        expression: RuleExpression::Read {
                            input: key("incoming"),
                        },
                    }],
                    effects: vec![RuleEffect {
                        id: key("factor"),
                        when: None,
                        effect: RuleEffectKind::Derive {
                            entity: RuleEntity::Current,
                            stat: output.clone(),
                            value: key("factor"),
                        },
                    }],
                }]);
                self.recipe.rules.receivers.members.push(StatReceiver {
                    id: program.clone(),
                    stat: output,
                    program,
                    targets: vec![StatReceiverTarget::Player],
                });
            }
        }
        fn try_plan(
            &self,
        ) -> std::result::Result<OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>, PlanError>
        {
            let scenario = ScenarioInput {
                game_version: ns(),
                enemy: EnemySpec {
                    encounter: def(0x31d1),
                    level: 20,
                },
                assumptions: vec![],
                usage: vec![],
            };
            empty_support::compile(&self.recipe, &self.build, &scenario, def(2))
        }
        fn evaluate(&self) -> OwnedEffectsReport {
            let p = self.try_plan().unwrap();
            effects(p.evaluate(&mut p.new_scratch()).unwrap())
        }
        fn edit(&mut self) {
            let a = self
                .build
                .allocations
                .iter_mut()
                .find(|a| a.node == crate::count_native::choice_control_node())
                .unwrap();
            assert_eq!(a.choices[0].value, ParameterValue::Option(def(0x1bf4)));
            a.choices[0].value = ParameterValue::Option(def(0x1bf2));
        }
    }
    fn effects(report: SupportEffectsReport) -> OwnedEffectsReport {
        assert!(report.gaps.is_empty(), "{:?}", report.gaps);
        let SupportEffectsOutcome::Evaluated { effects } = report.outcome else {
            panic!("{report:?}")
        };
        effects
    }
    fn value(report: &OwnedEffectsReport, stat: StatDefId) -> &EffectValue {
        &report
            .values
            .iter()
            .find(|v| {
                v.key
                    == PlanValueKey::Stat {
                        entity: ConcreteEntity::Actor(ActorKey::Player),
                        stat: stat.clone(),
                    }
            })
            .unwrap()
            .value
    }
    fn assert_attributes(report: &OwnedEffectsReport, expected: [i64; 3]) {
        for (i, n) in expected.iter().enumerate() {
            for id in [0x3321 + i as u64, 0x1d2e + i as u64] {
                assert_eq!(
                    value(report, def(id)),
                    &EffectValue::Known {
                        value: ParameterValue::Integer(BoundedInteger::new(*n).unwrap())
                    }
                );
            }
        }
    }
    fn assert_source_occurrences(world: &World, report: &OwnedEffectsReport, case: usize) {
        let registry = world.recipe.rules.contribution_queries.as_ref().unwrap();
        for stage in 0..6 {
            let query = &registry.members[stage * 2];
            let mut actual = Vec::new();
            for effect in &report.effects {
                let BoundEffectTarget::Contribution { key: target } = &effect.target else {
                    continue;
                };
                if target.stat != query.stat || target.kind != ContributionKind::Add {
                    continue;
                }
                let EffectValue::Known {
                    value: ParameterValue::Quantity(amount),
                } = &effect.value
                else {
                    assert_eq!(effect.value, EffectValue::Inactive);
                    continue;
                };
                assert_eq!(amount.unit(), &def(0x295a));
                let invocation = &effect.key.invocation;
                let RuleOrigin::Provider { provider } = &invocation.origin else {
                    panic!("ordinary original occurrence");
                };
                assert!(provider.grant_path.is_empty());
                let expected_owner = match &provider.root {
                    ProviderRoot::Character => {
                        SchemaSubject::Definition(world.build.character.class.address())
                    }
                    ProviderRoot::Allocation(id) => SchemaSubject::Definition(
                        world
                            .build
                            .allocations
                            .iter()
                            .find(|a| a.id == *id)
                            .unwrap()
                            .node
                            .address(),
                    ),
                    _ => panic!("source witness has only class and allocated choices"),
                };
                assert_eq!(invocation.owner, expected_owner);
                let member = query.groups[0]
                    .members
                    .members
                    .iter()
                    .find(|m| {
                        m.owner == invocation.owner
                            && m.program == invocation.program
                            && m.effect == effect.key.effect
                    })
                    .unwrap();
                actual.push((
                    member.order.as_ref().unwrap().source_rank,
                    world.source_name(&invocation.owner),
                    amount.value(),
                ));
            }
            actual.sort_by_key(|row| row.0);
            for mode in ["MAIN", "CALCS"] {
                let expected = rows(
                    &source().vectors["cases"][case]["modes"][mode]["stages"][stage]["records"],
                );
                assert_eq!(actual.len(), expected.len());
                for ((_, name, amount), row) in actual.iter().zip(expected) {
                    assert_eq!(name, row["source"].as_str().unwrap());
                    assert_eq!(*amount, row["value"].as_f64().unwrap());
                }
            }
        }
    }
    #[test]
    #[ignore = "requires verified ATTRIBUTE_STEP_RELEASE and immutable ATTRIBUTE_COUNT_RELEASE"]
    fn actual_original05_count_occurrences_flow_through_both_attribute_passes() {
        let mut w = World::new();
        assert_eq!(w.build.allocations.len(), 22);
        let baseline = w.evaluate();
        assert_attributes(&baseline, [27, 7, 105]);
        assert_source_occurrences(&w, &baseline, 0);
        let actual_ids: BTreeSet<_> = w.build.allocations.iter().map(|a| a.id).collect();
        let observed: BTreeSet<_> = baseline
            .effects
            .iter()
            .filter_map(|e| match &e.key.invocation.origin {
                RuleOrigin::Provider { provider } => {
                    if let ProviderRoot::Allocation(id) = &provider.root {
                        Some(*id)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect();
        assert_eq!(observed, actual_ids);
        w.edit();
        let changed = w.evaluate();
        assert_attributes(&changed, [22, 12, 105]);
        assert_source_occurrences(&w, &changed, 1);
        let mut permuted = w.clone();
        for query in &mut permuted
            .recipe
            .rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members
        {
            for member in &mut query.groups[0].members.members {
                let order = member.order.as_mut().unwrap();
                order.source_rank = 100_000 - order.source_rank;
            }
        }
        assert_attributes(&permuted.evaluate(), [22, 12, 105]);
        let original = World::new().evaluate();
        assert_eq!(original, baseline);
    }
    #[test]
    #[ignore = "requires verified ATTRIBUTE_STEP_RELEASE and immutable ATTRIBUTE_COUNT_RELEASE"]
    fn unresolved_more_and_membership_never_become_production_defaults() {
        let mut w = World::new();
        let stat = def(0x3327);
        w.recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == subject(stat.clone()))
            .unwrap()
            .programs
            .members
            .clear();
        w.recipe.rules.receivers.members.retain(|r| r.stat != stat);
        let r = w.evaluate();
        assert!(matches!(
            value(&r, def(0x1d2e)),
            EffectValue::Unresolved {
                reason: PlanGapReason::MissingProducer,
                ..
            }
        ));
        let mut unassigned = World::new();
        unassigned
            .recipe
            .rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members[0]
            .groups[0]
            .members
            .members
            .pop()
            .unwrap();
        let error = unassigned
            .try_plan()
            .err()
            .expect("unassigned actual potential contribution must fail");
        assert!(
            matches!(error,PlanError::Invalid(message) if message.contains("no ordered membership"))
        );
        let mut partial = World::new();
        let production: DeclaredSet<ContributionQuery> = read("queries.json");
        partial
            .recipe
            .rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members[0]
            .groups[0]
            .members
            .closure = production.members[0].groups[0].members.closure.clone();
        let p = partial.try_plan().unwrap();
        let r = p.evaluate(&mut p.new_scratch()).unwrap();
        assert!(
            r.gaps
                .iter()
                .any(|g| g.reason == PlanGapReason::IncompleteContributors)
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
        let mut actual = World::new();
        let published: Consumers = read("consumers.json");
        for original in published
            .owners
            .iter()
            .filter(|o| !o.programs.is_complete())
        {
            *actual
                .recipe
                .rules
                .owners
                .iter_mut()
                .find(|o| o.owner == original.owner)
                .unwrap() = original.clone();
        }
        actual
            .recipe
            .rules
            .receivers
            .members
            .retain(|r| !r.id.as_str().starts_with("fixture-empty-more-"));
        // An owner without a receiver is not instantiated solely because its
        // descriptor exists. Restoring the actual unimplemented factor owners
        // leaves demanded factors missing; it cannot supply an identity value.
        let report = actual.evaluate();
        for id in [0x3321, 0x3322, 0x3323, 0x1d2e, 0x1d2f, 0x1d30] {
            assert!(matches!(
                value(&report, def(id)),
                EffectValue::Unresolved {
                    reason: PlanGapReason::MissingProducer,
                    ..
                }
            ));
        }
    }
    #[test]
    #[ignore = "requires verified ATTRIBUTE_STEP_RELEASE and immutable ATTRIBUTE_COUNT_RELEASE"]
    fn shared_consumer_arithmetic_keeps_lazy_reads_rounding_clamp_and_overflow() {
        let w = World::new();
        let schema =
            OwnedDefinitionSchemaPackage::new(w.recipe.schema.clone(), Default::default()).unwrap();
        let mut rules = w.recipe.rules.clone();
        rules.definitions = schema.identity().clone();
        let compiled = CompiledRulePackage::compile(&rules, &schema, Default::default()).unwrap();
        let mut scratch = compiled.new_scratch();
        let b: Value = read("bindings.json");
        let stage = &b["stages"][0];
        let owner = subject(decode(&stage["output"]));
        let program = decode(&stage["program"]);
        let fact = |read: &str, n, unit| RuleFact {
            read: key(read),
            value: ParameterValue::Quantity(FiniteQuantity::new(n, def(unit)).unwrap()),
        };
        for (base, increase, more, expected) in [
            (-1.5, 0., 1., 0),
            (2.5, 0., 1., 3),
            (3.4999, 0., 1., 3),
            (-2. * BoundedInteger::MAX as f64, 0., 1., 0),
            (10., 10., 1.2, 13),
            (147., 6., 1., 156),
        ] {
            let result = compiled
                .evaluate(
                    &owner,
                    &program,
                    &[
                        fact("base", base, 0x295a),
                        fact("increase", increase, 2),
                        fact("effective-more", more, 1),
                    ],
                    &schema,
                    &mut scratch,
                )
                .unwrap();
            assert_eq!(
                result.effects[0].disposition,
                EffectDisposition::Applied {
                    value: ParameterValue::Integer(BoundedInteger::new(expected).unwrap())
                }
            );
        }
        let lazy = compiled
            .evaluate(
                &owner,
                &program,
                &[fact("base", 0., 0x295a)],
                &schema,
                &mut scratch,
            )
            .unwrap();
        assert_eq!(
            lazy.effects[0].disposition,
            EffectDisposition::Applied {
                value: ParameterValue::Integer(BoundedInteger::new(0).unwrap())
            }
        );
        let missing = compiled
            .evaluate(
                &owner,
                &program,
                &[fact("base", 7., 0x295a), fact("increase", 0., 2)],
                &schema,
                &mut scratch,
            )
            .unwrap();
        assert!(
            matches!(&missing.effects[0].disposition,EffectDisposition::Unresolved {input} if input.as_str()=="effective-more")
        );
        let overflow = compiled
            .evaluate(
                &owner,
                &program,
                &[
                    fact("base", BoundedInteger::MAX as f64 + 1., 0x295a),
                    fact("increase", 0., 2),
                    fact("effective-more", 1., 1),
                ],
                &schema,
                &mut scratch,
            )
            .unwrap();
        assert!(matches!(
            &overflow.effects[0].disposition,
            EffectDisposition::NumericalError {
                reason: NumericalFailure::IntegerOverflow,
                ..
            }
        ));
    }
    #[test]
    #[ignore = "requires verified ATTRIBUTE_STEP_RELEASE and immutable ATTRIBUTE_COUNT_RELEASE"]
    fn final_attribute_steps_are_deterministic_with_reused_and_parallel_scratch() {
        let a = World::new();
        let mut b = a.clone();
        b.edit();
        let pa = a.try_plan().unwrap();
        let pb = b.try_plan().unwrap();
        let va = a.evaluate();
        let vb = b.evaluate();
        assert_ne!(pa.identity(), pb.identity());
        let mut scratch = pa.new_scratch();
        for (p, v) in [(&pa, &va), (&pb, &vb), (&pa, &va)] {
            assert_eq!(&effects(p.evaluate(&mut scratch).unwrap()), v);
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
}
