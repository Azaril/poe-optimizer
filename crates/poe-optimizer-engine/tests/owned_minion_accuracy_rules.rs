//! Ordinary CI coverage of current authored rule laws. Explicit RuleFacts are
//! resolved component inputs, never proof of provider/query membership. The
//! joined root target tests actual imports, membership, routing and staging.
#[path = "support/minion_attack_source_fixture.rs"]
mod intrinsic;

use intrinsic::{def, key, quantity, repository_root};
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_rules::*,
    owned_schema::{DeclaredSet, DefinitionDescriptor, SchemaSubject},
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::owned_rules::*;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;

fn read<T: DeserializeOwned>(path: &str) -> T {
    serde_json::from_slice(&fs::read(repository_root().join(path)).unwrap()).unwrap()
}
fn asset<T: DeserializeOwned>(family: &str, file: &str) -> T {
    read(&format!("data/owned/poe2/3887ae68/{family}/{file}"))
}
fn decode<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
fn fact(read: &str, value: ParameterValue) -> RuleFact {
    RuleFact {
        read: key(read),
        value,
    }
}

struct Rules {
    schema: OwnedDefinitionSchemaPackage,
    compiled: CompiledRulePackage,
    owners: Vec<DefinitionRules>,
}
impl Rules {
    fn new() -> Self {
        // Reuse only the existing admitted schema. Its build, final-level
        // fixture producer, evaluator and staged plan are never compiled here.
        let mut schema = intrinsic::World::new([22, 1]).f.schema;
        let namespace = schema.namespace.clone();
        let mut owners = vec![];
        for family in ["configuration-block-inputs", "minion-accuracy"] {
            let mut definitions: Vec<DefinitionDescriptor> = asset(family, "dependencies.json");
            let extension: Value = asset(family, "extension.json");
            definitions.extend(extension["schema"].as_array().unwrap().iter().map(|row| {
                assert_eq!(row["kind"], "definition");
                decode(&row["value"])
            }));
            for definition in definitions {
                schema
                    .definitions
                    .retain(|d| d.address() != definition.address());
                schema.definitions.push(definition);
            }
            owners.extend(decode::<Vec<DefinitionRules>>(&extension["owners"]));
        }
        for definition in
            asset::<Vec<DefinitionDescriptor>>("minion-accuracy-flags", "schemas.json")
        {
            schema
                .definitions
                .retain(|d| d.address() != definition.address());
            schema.definitions.push(definition);
        }
        let replacements: Value = asset("minion-accuracy-flags", "programs.json");
        assert_eq!(replacements["programs"].as_array().unwrap().len(), 2);
        for row in replacements["programs"].as_array().unwrap() {
            let owner: SchemaSubject = decode(&row["owner"]);
            let program: RuleProgram = decode(&row["program"]);
            let target = owners.iter_mut().find(|o| o.owner == owner).unwrap();
            assert!(!target.programs.is_complete());
            let at = target
                .programs
                .members
                .iter()
                .position(|p| p.id == program.id)
                .unwrap();
            target.programs.members[at] = program;
        }
        assert_eq!(
            owners
                .iter()
                .map(|o| o.programs.members.len())
                .sum::<usize>(),
            3
        );
        let queries: Vec<ContributionQuery> = asset("minion-accuracy-flags", "queries.json");
        assert_eq!(queries.len(), 2);
        for query in &queries {
            assert_eq!(query.contribution, ContributionKind::Flag);
            assert!(query.groups.iter().all(|g| !g.members.is_complete()));
        }
        let schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
        let rules = RulePackageInput {
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace,
            release: key("ordinary-accuracy-rule-test"),
            semantics_version: key("ordinary-accuracy-rule-test"),
            operations_version: key(OWNED_RULE_OPERATIONS_V22),
            definitions: schema.identity().clone(),
            tables: vec![],
            owners: owners.clone(),
            receivers: DeclaredSet::complete(vec![]),
            effect_applications: Some(DeclaredSet::complete(vec![])),
            // Only registry discovery is finite here. Both actual query member
            // declarations remain Partial; direct RuleFacts do not close them.
            contribution_queries: Some(DeclaredSet::complete(queries)),
            existing_actor_rules: None,
        };
        let compiled = CompiledRulePackage::compile(&rules, &schema, Default::default()).unwrap();
        Self {
            schema,
            compiled,
            owners,
        }
    }
    fn evaluate(
        &self,
        name: &str,
        facts: &[RuleFact],
        scratch: &mut RuleScratch,
    ) -> ProgramEvaluation {
        let matches: Vec<_> = self
            .owners
            .iter()
            .filter(|o| o.programs.members.iter().any(|p| p.id == key(name)))
            .collect();
        assert_eq!(matches.len(), 1);
        let result = self
            .compiled
            .evaluate(&matches[0].owner, &key(name), facts, &self.schema, scratch)
            .unwrap();
        assert_eq!(result.owner_programs_closure, matches[0].programs.closure);
        result
    }
}
fn disposition<'a>(report: &'a ProgramEvaluation, id: &str) -> &'a EffectDisposition {
    let rows: Vec<_> = report.effects.iter().filter(|e| e.id == key(id)).collect();
    assert_eq!(rows.len(), 1);
    &rows[0].disposition
}
fn applied(report: &ProgramEvaluation, id: &str) -> ParameterValue {
    let EffectDisposition::Applied { value } = disposition(report, id) else {
        panic!("expected applied {id}: {report:?}")
    };
    value.clone()
}
fn close(actual: &ParameterValue, expected: &Value) {
    let ParameterValue::Quantity(value) = actual else {
        panic!("quantity")
    };
    assert_eq!(value.unit(), &def(2));
    assert!((value.value() - expected.as_f64().unwrap()).abs() < 1e-10);
}

#[test]
fn current_boolean_accuracy_rules_replay_thirteen_measured_inputs() {
    let rules = Rules::new();
    let vectors: Value = read("tests/fixtures/calibration/minion-accuracy-3887ae68.json");
    let authoring: Value = asset("minion-accuracy", "authoring.json");
    let manifest =
        fs::read(repository_root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json"))
            .unwrap();
    assert_eq!(
        vectors["source_hash"],
        format!("{:x}", Sha256::digest(&manifest))
    );
    assert_eq!(vectors["source_hash"], authoring["source_manifest_sha256"]);
    assert_eq!(vectors["source_revision"], authoring["source_revision"]);
    let rows = vectors["sniper"].as_array().unwrap();
    assert_eq!(rows.len(), 13);
    let mut scratch = rules.compiled.new_scratch();
    for row in rows {
        let pass = &row["consumer"]["passes"][0];
        let raw = &row["config"]["enemy_block"];
        let selected = if raw["input_present"] == true {
            Some(raw["input"].as_f64().unwrap())
        } else if raw["placeholder_present"] == true {
            Some(raw["placeholder"].as_f64().unwrap())
        } else {
            None
        };
        let mut input = vec![fact("present", ParameterValue::Boolean(selected.is_some()))];
        if let Some(value) = selected {
            input.push(fact("raw", quantity(value, 2)));
        }
        let config = rules.evaluate("configured-enemy-block-base", &input, &mut scratch);
        let block = applied(&config, "configured-base");
        close(&block, &pass["block"]["base"]);
        let inheritance = pass["flags"]["player_minion_accuracy_equals_accuracy"]
            .as_bool()
            .unwrap();
        assert!(!inheritance);
        let actor = rules.evaluate(
            "intrinsic-minion-cannot-be-evaded",
            &[fact(
                "inheritance-flags",
                ParameterValue::Boolean(inheritance),
            )],
            &mut scratch,
        );
        assert_eq!(
            applied(&actor, "cannot-be-evaded"),
            ParameterValue::Boolean(true)
        );
        let accuracy = applied(&actor, "accuracy-hit-chance");
        close(&accuracy, &pass["output"]["AccuracyHitChance"]);
        assert_eq!(pass["block"]["reduction"], 0);
        assert!(
            pass["block"]["reduction_records"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        // These are explicit resolved query facts for rule-law coverage. Custom
        // cannot-block and source raw40 recovery do not gain native build/input
        // admission merely because their independent hit-chance law is covered.
        let facts = vec![
            fact("accuracy", accuracy),
            fact("block-base", block),
            fact("block-reduction", quantity(0., 2)),
            fact(
                "cannot-block-flags",
                ParameterValue::Boolean(
                    pass["flags"]["enemy_cannot_block_attacks"]
                        .as_bool()
                        .unwrap(),
                ),
            ),
        ];
        let action = rules.evaluate("ordinary-minion-attack-hit-chance", &facts, &mut scratch);
        close(
            &applied(&action, "effective-enemy-block"),
            &pass["output"]["enemyBlockChance"],
        );
        close(
            &applied(&action, "hit-chance"),
            &pass["output"]["HitChance"],
        );
        assert_eq!(
            action,
            rules.evaluate(
                "ordinary-minion-attack-hit-chance",
                &facts,
                &mut rules.compiled.new_scratch()
            )
        );
    }
}

#[test]
fn current_boolean_rules_preserve_missing_facts_inheritance_and_scratch_isolation() {
    let rules = Rules::new();
    let mut scratch = rules.compiled.new_scratch();
    let facts = vec![
        fact("accuracy", quantity(100., 2)),
        fact("block-base", quantity(37., 2)),
        fact("block-reduction", quantity(0., 2)),
        fact("cannot-block-flags", ParameterValue::Boolean(false)),
    ];
    let baseline = rules.evaluate("ordinary-minion-attack-hit-chance", &facts, &mut scratch);
    close(&applied(&baseline, "hit-chance"), &json!(63));
    let missing = rules.evaluate(
        "ordinary-minion-attack-hit-chance",
        &facts[..3],
        &mut scratch,
    );
    assert_eq!(
        disposition(&missing, "hit-chance"),
        &EffectDisposition::Unresolved {
            input: key("cannot-block-flags")
        }
    );
    let inherited = rules.evaluate(
        "intrinsic-minion-cannot-be-evaded",
        &[fact("inheritance-flags", ParameterValue::Boolean(true))],
        &mut scratch,
    );
    assert_eq!(
        applied(&inherited, "cannot-be-evaded"),
        ParameterValue::Boolean(false)
    );
    assert_eq!(
        disposition(&inherited, "accuracy-hit-chance"),
        &EffectDisposition::Inactive
    );
    let unknown = rules.evaluate("intrinsic-minion-cannot-be-evaded", &[], &mut scratch);
    assert_eq!(
        disposition(&unknown, "cannot-be-evaded"),
        &EffectDisposition::Unresolved {
            input: key("inheritance-flags")
        }
    );
    for inputs in [vec![], vec![fact("present", ParameterValue::Boolean(true))]] {
        let config = rules.evaluate("configured-enemy-block-base", &inputs, &mut scratch);
        assert!(matches!(
            disposition(&config, "configured-base"),
            EffectDisposition::Unresolved { .. }
        ));
    }
    let owner = &rules
        .owners
        .iter()
        .find(|o| {
            o.programs
                .members
                .iter()
                .any(|p| p.id == key("intrinsic-minion-cannot-be-evaded"))
        })
        .unwrap()
        .owner;
    assert!(
        rules
            .compiled
            .evaluate(
                owner,
                &key("intrinsic-minion-cannot-be-evaded"),
                &[fact("inheritance-flags", quantity(1., 2))],
                &rules.schema,
                &mut scratch
            )
            .is_err()
    );
    assert_eq!(
        baseline,
        rules.evaluate("ordinary-minion-attack-hit-chance", &facts, &mut scratch)
    );
}
