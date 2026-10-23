//! Exact per-occurrence Count cutover. This supplies neither final attributes nor ordering policy.
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_import::owned_release_migration::OwnedReleaseMigrationInput;
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};
#[derive(Deserialize)]
struct Programs {
    schema_version: u32,
    programs: Vec<Replacement>,
}
#[derive(Deserialize)]
struct Replacement {
    owner: SchemaSubject,
    closure: SchemaClosure,
    before: RuleProgram,
    after: RuleProgram,
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/attribute-stages")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(name: &str) -> OwnedDefinitionKey {
    name.parse().unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn channels(b: &Value) -> (UnitDefId, Vec<StatDefId>, Vec<Vec<StatDefId>>) {
    (
        serde_json::from_value(b["unit"].clone()).unwrap(),
        serde_json::from_value(b["final_attributes"].clone()).unwrap(),
        b["passes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| serde_json::from_value(p["inputs"].clone()).unwrap())
            .collect(),
    )
}
fn targets(program: &RuleProgram, old: &[StatDefId]) -> Vec<(usize, usize)> {
    program
        .effects
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let RuleEffectKind::Contribute { stat, .. } = &e.effect else {
                return None;
            };
            old.iter()
                .position(|s| s == stat)
                .map(|attribute| (i, attribute))
        })
        .collect()
}
fn expected(row: &Replacement, b: &Value) -> RuleProgram {
    let (unit, old, passes) = channels(b);
    let affected = targets(&row.before, &old);
    assert!(!affected.is_empty());
    let mut after = row.before.clone();
    let modifier = matches!(
        &row.owner,
        SchemaSubject::Definition(DefinitionAddress::Modifier(_))
    );
    if modifier {
        assert_eq!(row.before.id.as_str(), "contribute-player-attributes");
        assert_eq!(row.before.nodes.len(), 2);
        assert_eq!(row.before.nodes[1].id.as_str(), "integer");
        assert_eq!(
            row.before.nodes[1].expression,
            RuleExpression::QuantizeInteger {
                value: key("effective"),
                quantum: FiniteQuantity::new(1., unit.clone()).unwrap(),
                mode: RuleRounding::Floor,
            }
        );
        after.nodes.extend([
            RuleNode {
                id: key("one-count"),
                expression: RuleExpression::Literal {
                    value: ParameterValue::Quantity(FiniteQuantity::new(1., unit.clone()).unwrap()),
                },
            },
            RuleNode {
                id: key("count"),
                expression: RuleExpression::ScaleInteger {
                    value: key("one-count"),
                    count: key("integer"),
                },
            },
        ]);
    } else {
        assert!(matches!(
            &row.owner,
            SchemaSubject::Definition(
                DefinitionAddress::Class(_) | DefinitionAddress::PassiveNode(_)
            )
        ));
        let mut converted = BTreeSet::new();
        for (index, _) in &affected {
            let RuleEffectKind::Contribute {
                contribution,
                value,
                ..
            } = &row.before.effects[*index].effect
            else {
                unreachable!()
            };
            if *contribution != ContributionKind::Add {
                assert_eq!(*contribution, ContributionKind::Increase);
                continue;
            }
            if !converted.insert(value.clone()) {
                continue;
            }
            let node = after
                .nodes
                .iter_mut()
                .find(|node| &node.id == value)
                .unwrap();
            let RuleExpression::Literal {
                value: ParameterValue::Integer(integer),
            } = &node.expression
            else {
                panic!("reviewed direct integer literal")
            };
            let quantity = ParameterValue::Quantity(
                FiniteQuantity::new(integer.get() as f64, unit.clone()).unwrap(),
            );
            node.expression = RuleExpression::Literal { value: quantity };
            // A converted literal cannot alter a condition, read or unrelated effect.
            for n in &row.before.nodes {
                assert!(
                    !json!(n.expression)
                        .as_object()
                        .unwrap()
                        .values()
                        .any(|v| v.as_str() == Some(value.as_str()))
                );
            }
            for effect in &row.before.effects {
                if let RuleEffectKind::Contribute {
                    stat,
                    contribution,
                    value: other,
                    ..
                } = &effect.effect
                    && other == value
                {
                    assert!(old.contains(stat));
                    assert_eq!(*contribution, ContributionKind::Add);
                } else {
                    assert!(
                        !json!(effect.effect)
                            .as_object()
                            .unwrap()
                            .values()
                            .any(|v| v.as_str() == Some(value.as_str()))
                    );
                }
            }
        }
    }
    after.effects.clear();
    for (index, effect) in row.before.effects.iter().enumerate() {
        let Some((_, attribute)) = affected.iter().find(|(i, _)| *i == index) else {
            after.effects.push(effect.clone());
            continue;
        };
        for (pass, inputs) in passes.iter().enumerate() {
            let mut e = effect.clone();
            if pass == 1 {
                e.id = key(&format!("{}-pass-two", e.id));
            }
            let RuleEffectKind::Contribute { stat, value, .. } = &mut e.effect else {
                unreachable!()
            };
            *stat = inputs[*attribute].clone();
            if modifier {
                *value = key("count");
            }
            after.effects.push(e);
        }
    }
    let ids: BTreeSet<_> = after.effects.iter().map(|e| &e.id).collect();
    assert_eq!(ids.len(), after.effects.len());
    after
}
pub fn check_authored() {
    let a: Value = read("count-authoring.json");
    let b: Value = read("count-bindings.json");
    let p: Programs = read("count-programs.json");
    let m: OwnedReleaseMigrationInput = read("count-schema-migration.json");
    assert_eq!(a["schema_version"], 1);
    assert_eq!(b["schema_version"], 1);
    assert_eq!(p.schema_version, 1);
    assert_eq!(a["before"], b["before"]);
    assert_eq!(json!(m.before), b["before"]);
    for (name, entry) in a["artifacts"].as_object().unwrap() {
        let bytes = fs::read(data().join(name)).unwrap();
        assert_eq!(bytes.len() as u64, entry["bytes"].as_u64().unwrap());
        assert_eq!(hash(&bytes), entry["sha256"]);
        assert!(bytes.ends_with(b"\n") && !bytes.contains(&b'\r'));
    }
    assert_eq!(a["artifacts"].as_object().unwrap().len(), 3);
    let (unit, old, passes) = channels(&b);
    assert_eq!(unit.key().as_str(), "def.000000000000295a");
    assert_eq!(old.len(), 3);
    assert_eq!(passes.len(), 2);
    for (i, s) in old.iter().enumerate() {
        assert_eq!(s.key().as_str(), format!("def.{:016x}", 0x1d2e + i));
    }
    for (pass, inputs) in passes.iter().enumerate() {
        assert_eq!(inputs.len(), 3);
        for (i, s) in inputs.iter().enumerate() {
            assert_eq!(
                s.key().as_str(),
                format!("def.{:016x}", 0x331b + pass * 3 + i)
            );
        }
    }
    assert_eq!(m.schema_version, 5);
    assert_eq!(
        m.contract.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_V20
    );
    assert_eq!(m.schema.len(), 6);
    assert!(
        m.owners.is_empty()
            && m.tables.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    for (entry, id) in m.schema.iter().zip(passes.iter().flatten()) {
        assert_eq!(
            json!(entry),
            json!({"kind":"definition","value":{"kind":"stat","value":{"id":id,"schema":{"kind":"known","value":{"value":{"kind":"quantity","value":{"unit":unit}},"targets":["actor"]}}}}})
        );
    }
    assert_eq!(p.programs.len(), 368);
    let mut identities = BTreeSet::new();
    let mut families = BTreeMap::new();
    let mut add = 0;
    let mut increase = 0;
    let mut emitted = 0;
    for row in &p.programs {
        assert!(identities.insert((
            serde_json::to_string(&row.owner).unwrap(),
            row.before.id.clone()
        )));
        assert_eq!(
            row.after,
            expected(row, &b),
            "exact occurrence cutover differs for {}",
            row.before.id
        );
        *families.entry(row.before.id.as_str()).or_insert(0_usize) += 1;
        let is_passive = matches!(
            &row.owner,
            SchemaSubject::Definition(DefinitionAddress::PassiveNode(_))
        );
        assert_eq!(matches!(row.closure, SchemaClosure::Complete), is_passive);
        for (i, _) in targets(&row.before, &old) {
            match &row.before.effects[i].effect {
                RuleEffectKind::Contribute {
                    contribution: ContributionKind::Add,
                    ..
                } => add += 1,
                RuleEffectKind::Contribute {
                    contribution: ContributionKind::Increase,
                    ..
                } => increase += 1,
                _ => panic!("only existing Add/Increase"),
            }
            emitted += 2;
        }
        assert!(targets(&row.after, &old).is_empty());
    }
    assert_eq!((add, increase, emitted), (992, 17, 2018));
    assert_eq!(json!(families), b["program_families"]);
    for (field, n) in [
        ("source_programs", 368),
        ("source_add_effects", 992),
        ("source_increase_effects", 17),
        ("new_definitions", 6),
        ("new_receivers", 0),
        ("new_ordered_queries", 0),
        ("closed_rule_owners", 0),
    ] {
        assert_eq!(a[field], n);
    }
    assert_eq!(a["final_attribute_claim"], false);
    assert_eq!(a["stage_algorithm_claim"], false);
}
/// Authenticate our exact bodies inside a later larger authored recipe as well.
pub fn assert_programs(rules: &RulePackageInput) {
    let p: Programs = read("count-programs.json");
    let b: Value = read("count-bindings.json");
    let (_, old, _) = channels(&b);
    for row in &p.programs {
        let owners: Vec<_> = rules
            .owners
            .iter()
            .filter(|o| o.owner == row.owner)
            .collect();
        assert_eq!(owners.len(), 1);
        assert_eq!(owners[0].programs.closure, row.closure);
        let programs: Vec<_> = owners[0]
            .programs
            .members
            .iter()
            .filter(|p| p.id == row.after.id)
            .collect();
        assert_eq!(programs.len(), 1);
        assert_eq!(programs[0], &row.after);
    }
    assert!(
        rules.owners.iter().all(|o| o
            .programs
            .members
            .iter()
            .all(|p| targets(p, &old).is_empty())),
        "old Integer contribution path remains live"
    );
}
