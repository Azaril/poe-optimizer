//! Test-only conversion of explicit fixture records into the public native API.
use super::oracle::Observed;
use poe_optimizer_engine::{
    conditions::{
        ConditionProgram, ConditionProgramActor, ConditionProgramInput, ConditionQuery,
        ConditionResult, ConditionStoreInput, ConditionValue, ConditionVariables,
        FlagModifierInput, FlagTag, ModifierTag, ScalarConditions, WeaponConditions,
    },
    modifiers::{
        ModifierDatabase, ModifierInput, ModifierKind, ModifierLayerInput, ModifierStoreKind,
        ModifierValue, MorePrecision, NumericKind, QueryContext, SumKind, TaggedModifierInput,
    },
    multipliers::{ScalarSource, StatThreshold, StatThresholdValue, StatVariables},
    stats::{ResolvedStatEnvironment, StatValues},
};
use serde_json::Value;
use std::collections::BTreeMap;

fn scalar(value: &Value) -> ConditionValue {
    match value {
        Value::Null => ConditionValue::Nil,
        Value::Bool(value) => ConditionValue::Boolean(*value),
        Value::Number(value) => ConditionValue::Number(value.as_f64().unwrap()),
        Value::String(value) => ConditionValue::Text(value.clone()),
        Value::Array(_) | Value::Object(_) => ConditionValue::Unsupported("table".into()),
    }
}
fn values(value: &Value) -> ScalarConditions {
    value
        .as_object()
        .into_iter()
        .flatten()
        .map(|(name, value)| (name.clone(), scalar(value)))
        .collect()
}
fn stats(value: &Value) -> StatValues {
    value
        .as_object()
        .into_iter()
        .flatten()
        .map(|(name, value)| (name.clone(), value.as_f64().unwrap()))
        .collect()
}
fn variables(tag: &Value) -> Option<ConditionVariables> {
    if let Some(list) = tag.get("varList") {
        Some(ConditionVariables::Any(
            list.as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().into())
                .collect(),
        ))
    } else {
        tag["var"]
            .as_str()
            .map(|v| ConditionVariables::One(v.into()))
    }
}
fn tag(tag: &Value) -> FlagTag {
    let negated = tag["neg"].as_bool().unwrap_or(false);
    match tag["type"].as_str().unwrap() {
        "Condition" => FlagTag::Predicate(ModifierTag::Condition {
            variables: variables(tag).unwrap(),
            negated,
        }),
        "ActorCondition" => FlagTag::Predicate(ModifierTag::ActorCondition {
            actor: tag["actor"].as_str().map(str::to_owned),
            variables: variables(tag),
            negated,
        }),
        "Global" => FlagTag::Predicate(ModifierTag::Global),
        "GlobalEffect" => FlagTag::Predicate(ModifierTag::GlobalEffect {
            effect_type: tag["effectType"].as_str().unwrap().into(),
            unscalable: tag["unscalable"].as_bool().unwrap(),
        }),
        "StatThreshold" => FlagTag::StatThreshold(StatThreshold {
            stats: if let Some(list) = tag["statList"].as_array() {
                StatVariables::Sum(list.iter().map(|v| v.as_str().unwrap().into()).collect())
            } else {
                StatVariables::One(tag["stat"].as_str().unwrap().into())
            },
            threshold: if let Some(value) = tag["threshold"].as_f64() {
                StatThresholdValue::Constant(value)
            } else {
                StatThresholdValue::Stat(tag["thresholdStat"].as_str().unwrap().into())
            },
            percent: tag["thresholdPercent"]
                .as_f64()
                .map(ScalarSource::Constant)
                .or_else(|| {
                    tag["thresholdPercentVar"]
                        .as_str()
                        .map(|v| ScalarSource::Multiplier(v.into()))
                }),
            upper: tag["upper"].as_bool().unwrap_or(false),
        }),
        kind => FlagTag::Unsupported(kind.into()),
    }
}
fn weapon(value: &Value) -> WeaponConditions {
    WeaponConditions {
        counts_as_all_one_handed: value["countsAsAll1H"].as_bool().unwrap_or(false),
        added: value
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(name, value)| {
                name.strip_prefix("Added")
                    .filter(|_| !value.is_null())
                    .map(|name| {
                        (
                            name.to_owned(),
                            value
                                .as_bool()
                                .expect("fixture weapon values are explicitly boolean"),
                        )
                    })
            })
            .collect(),
    }
}

pub fn query(input: &Value) -> Result<Observed, String> {
    let stores = input["stores"].as_array().unwrap();
    let actors = input["actors"].as_array().unwrap();
    let program = ConditionProgram::try_new(ConditionProgramInput {
        stores: stores
            .iter()
            .map(|store| ConditionStoreInput {
                kind: match store["store_type"].as_str() {
                    None | Some("ModDB") => ModifierStoreKind::ModDb,
                    Some("ModList") => ModifierStoreKind::ModList,
                    Some(other) => panic!("Unrepresented source store kind: {other}"),
                },
                parent: store["parent"].as_u64().map(|v| v as usize - 1),
                actor: store["actor"].as_u64().unwrap() as usize - 1,
                flags: store["mods"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|record| record["type"] == "FLAG")
                    .map(|record| {
                        assert_eq!(record["type"], "FLAG");
                        FlagModifierInput {
                            name: record["name"].as_str().unwrap().into(),
                            value: scalar(&record["value"]),
                            flags: record["flags"].as_u64().unwrap(),
                            keyword_flags: record["keywordFlags"].as_u64().unwrap(),
                            source: record["source"].as_str().map(str::to_owned),
                            tags: record["tags"].as_array().unwrap().iter().map(tag).collect(),
                        }
                    })
                    .collect(),
                unsupported_features: vec![],
            })
            .collect(),
        actors: actors
            .iter()
            .map(|actor| ConditionProgramActor {
                store: actor["store"].as_u64().unwrap() as usize - 1,
                links: actor["links"]
                    .as_object()
                    .into_iter()
                    .flatten()
                    .map(|(role, index)| (role.clone(), index.as_u64().unwrap() as usize - 1))
                    .collect::<BTreeMap<_, _>>(),
                weapon_one: weapon(&actor["weapon_one"]),
                weapon_two: weapon(&actor["weapon_two"]),
                unsupported_features: vec![],
            })
            .collect(),
        unsupported_features: vec![],
    })
    .map_err(|e| e.to_string())?;
    let context = QueryContext {
        flags: input["cfg"]["flags"].as_u64().unwrap_or(0),
        keyword_flags: input["cfg"]["keywordFlags"].as_u64().unwrap_or(0),
        source: input["cfg"]["source"].as_str().map(str::to_owned),
    };
    let store_conditions: Vec<_> = stores.iter().map(|v| values(&v["conditions"])).collect();
    let overrides = values(&input["cfg"]["overrideCond"]);
    let skill_conditions = values(&input["cfg"]["skillCond"]);
    let stat_values: Vec<_> = stores
        .iter()
        .map(|store| {
            let actor = &actors[store["actor"].as_u64().unwrap() as usize - 1];
            ResolvedStatEnvironment::try_new(
                actor.get("output").map(stats),
                stats(&input["cfg"]["skillStats"]),
                vec![],
            )
            .unwrap()
        })
        .collect();
    let config = ConditionQuery {
        context: &context,
        store_conditions: &store_conditions,
        overrides: &overrides,
        skill_conditions: &skill_conditions,
        stats: &stat_values,
        query_actor: input["cfg"]["actor"].as_str(),
        ignore_source_in_check_conditions: input["cfg"]["ignoreSourceInCheckConditions"]
            .as_bool()
            .unwrap_or(false),
    };
    let bound = program
        .bind(input["root"].as_u64().unwrap() as usize - 1, &config)
        .map_err(|e| e.to_string())?;
    match input["query"]["kind"].as_str().unwrap() {
        "flag" => {
            let names: Vec<_> = input["query"]["names"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect();
            Ok(match bound.flag(&names).map_err(|e| e.to_string())? {
                Some(true) => Observed::Boolean(true),
                None => Observed::Nil,
                Some(false) => panic!("FLAG must return true or nil"),
            })
        }
        "condition" => Ok(
            match bound
                .get_condition(
                    input["query"]["variable"].as_str().unwrap(),
                    input["query"]["no_mod"].as_bool().unwrap_or(false),
                )
                .map_err(|e| e.to_string())?
            {
                ConditionResult::Nil => Observed::Nil,
                ConditionResult::Boolean(value) => Observed::Boolean(value),
                ConditionResult::Number(value) => Observed::Number(value),
                ConditionResult::Text(value) => Observed::Text(value.into()),
            },
        ),
        "sum" | "more" | "override" | "max" | "positive" => {
            let mut layers = Vec::new();
            let mut current = Some(input["root"].as_u64().unwrap() as usize - 1);
            while let Some(index) = current {
                let mut layer = Vec::new();
                for record in stores[index]["mods"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|r| r["type"] != "FLAG")
                {
                    let kind = match record["type"].as_str().unwrap() {
                        "BASE" => NumericKind::Base,
                        "INC" => NumericKind::Increased,
                        "MORE" => NumericKind::More,
                        "OVERRIDE" => NumericKind::Override,
                        "MAX" => NumericKind::Max,
                        other => return Err(format!("unsupported numeric fixture {other}")),
                    };
                    let tags = record["tags"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|value| match tag(value) {
                            FlagTag::Predicate(value) => Ok(value),
                            _ => Err("numeric fixture uses an unsupported predicate".to_owned()),
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    layer.push(TaggedModifierInput {
                        modifier: ModifierInput {
                            name: record["name"].as_str().unwrap().into(),
                            kind: ModifierKind::Numeric(kind),
                            value: numeric_value(record),
                            flags: record["flags"].as_u64().unwrap(),
                            keyword_flags: record["keywordFlags"].as_u64().unwrap(),
                            source: record["source"].as_str().map(str::to_owned),
                            tag_kinds: vec![],
                        },
                        tags,
                    });
                }
                layers.push(ModifierLayerInput {
                    kind: store_kind(&stores[index]),
                    modifiers: layer,
                });
                current = stores[index]["parent"].as_u64().map(|v| v as usize - 1);
            }
            let database = ModifierDatabase::try_new_layers(layers).map_err(|e| e.to_string())?;
            let names: Vec<_> = input["query"]["names"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect();
            let operation = input["query"]["kind"].as_str().unwrap();
            let optional = |value: Option<f64>| value.map_or(Observed::Nil, Observed::Number);
            let result = match operation {
                "sum" | "positive" => {
                    let kind = match input["query"]["operation"].as_str().unwrap() {
                        "BASE" => SumKind::Base,
                        "INC" => SumKind::Increased,
                        other => panic!("unknown fixture sum {other}"),
                    };
                    if operation == "sum" {
                        database.sum_with_conditions(kind, &context, &names, &bound)
                    } else {
                        database.sum_positive_with_conditions(kind, &context, names[0], &bound)
                    }
                    .map(Observed::Number)
                }
                "more" => {
                    let precision = MorePrecision::try_new(
                        input["precision"]
                            .as_object()
                            .into_iter()
                            .flatten()
                            .map(|(k, v)| (k.clone(), v.as_u64().unwrap() as u8))
                            .collect(),
                    )
                    .unwrap();
                    database
                        .more_with_conditions(&context, &names, &precision, &bound)
                        .map(Observed::Number)
                }
                "override" => database
                    .override_with_conditions(&context, &names, &bound)
                    .map(optional),
                "max" => database
                    .max_with_conditions(&context, &names, &bound)
                    .map(optional),
                _ => unreachable!(),
            };
            result.map_err(|e| e.to_string())
        }
        other => panic!("unsupported paired test operation {other}"),
    }
}

fn store_kind(store: &Value) -> ModifierStoreKind {
    match store["store_type"].as_str().unwrap_or("ModDB") {
        "ModDB" => ModifierStoreKind::ModDb,
        "ModList" => ModifierStoreKind::ModList,
        other => panic!("unknown test store {other}"),
    }
}
fn numeric_value(record: &Value) -> ModifierValue {
    if let Some(kind) = record["nonfinite_value"].as_str() {
        return ModifierValue::Number(match kind {
            "nan" => f64::NAN,
            "positive_infinity" => f64::INFINITY,
            "negative_infinity" => f64::NEG_INFINITY,
            _ => panic!("unknown nonfinite fixture"),
        });
    }
    if let Some(value) = record["value"].as_f64() {
        ModifierValue::Number(value)
    } else {
        ModifierValue::Unsupported {
            kind: if record["value"].is_boolean() {
                "boolean"
            } else {
                "nil"
            }
            .into(),
        }
    }
}
