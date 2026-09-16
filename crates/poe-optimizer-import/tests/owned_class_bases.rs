//! Pinned class facts plus independent values; no Lua or full-build authority.
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition};
use poe_optimizer_import::{owned_class_bases::*, owned_mapping::*, owned_recipe::*};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn load<T: DeserializeOwned>(relative: &str) -> T {
    serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68").join(relative)).unwrap(),
    )
    .unwrap()
}
fn rebind(recipe: &mut OwnedRecipeInput) {
    let schema = poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage::new(
        recipe.schema.clone(),
        Default::default(),
    )
    .unwrap();
    recipe.rules.definitions = schema.identity().clone();
    recipe.routing.definitions = schema.identity().clone();
}
struct Fixture {
    recipe: OwnedRecipeInput,
    mapping: MappingPackageInput,
    source: Vec<u8>,
    policy: ClassBaseRecipePolicy,
}
impl Fixture {
    fn new() -> Self {
        let mut recipe: OwnedRecipeInput = load("current/recipe.json");
        let mapping = load("current/mapping.json");
        let source_path = "src/TreeData/0_5/tree.json";
        let source = fs::read(
            root()
                .join("vendor/path-of-building-poe2")
                .join(source_path),
        )
        .unwrap();
        let mut registry =
            OwnedIdRegistry::new(recipe.registry.clone(), Default::default()).unwrap();
        let fields = ["base_str", "base_dex", "base_int"]
            .into_iter()
            .map(|source_field| {
                let stat: StatDefId = registry.allocate_definition().unwrap();
                recipe
                    .schema
                    .definitions
                    .push(DefinitionDescriptor::Stat(DefinitionEntry {
                        id: stat.clone(),
                        schema: SchemaState::Known(StatSchema {
                            value: ComputedValueType::Integer,
                            targets: vec![RuleEntityKind::Actor],
                        }),
                    }));
                ClassBaseFieldPolicy {
                    source_field: source_field.into(),
                    stat,
                }
            })
            .collect();
        recipe.registry = registry.input().clone();
        rebind(&mut recipe);
        Self {
            recipe,
            mapping,
            source,
            policy: ClassBaseRecipePolicy {
                schema_version: OWNED_CLASS_BASES_VERSION,
                version: key("reviewed-class-facts"),
                source_path: source_path.into(),
                expected_class_fields: [
                    "ascendancies",
                    "background",
                    "base_dex",
                    "base_int",
                    "base_str",
                    "integerId",
                    "name",
                ]
                .map(str::to_owned)
                .to_vec(),
                fields,
            },
        }
    }
    fn checked(&self) -> (StagedOwnedRecipe, OwnedMappingIndex) {
        let base = assemble_owned_recipe(self.recipe.clone(), Default::default()).unwrap();
        let mut input = self.mapping.clone();
        input.registry = base.registry().identity().unwrap();
        input.definitions = base.schema().identity().clone();
        let mapping =
            OwnedMappingIndex::new(input, base.registry(), base.schema(), Default::default())
                .unwrap();
        (base, mapping)
    }
    fn compile(&self) -> Result<StagedClassBaseRecipe, ClassBaseRecipeError> {
        let (base, mapping) = self.checked();
        compile_owned_class_bases(
            &base,
            &mapping,
            &self.source,
            &self.policy,
            Default::default(),
        )
    }
    fn repin(&mut self) {
        let normalized: Vec<_> = self
            .source
            .iter()
            .enumerate()
            .filter(|(i, b)| **b != b'\r' || self.source.get(i + 1) != Some(&b'\n'))
            .map(|(_, b)| *b)
            .collect();
        self.mapping
            .source
            .files
            .iter_mut()
            .find(|p| p.path == self.policy.source_path)
            .unwrap()
            .sha256 = format!("{:x}", Sha256::digest(normalized));
    }
    fn edit_source(&mut self, change: impl FnOnce(&mut Value)) {
        let mut source: Value = serde_json::from_slice(&self.source).unwrap();
        change(&mut source);
        self.source = serde_json::to_vec(&source).unwrap();
        self.repin();
    }
}
fn bad(f: &Fixture, message: &str) {
    let error = f.compile().unwrap_err();
    assert!(error.to_string().contains(message), "{error}");
}

#[test]
fn all_pinned_classes_emit_exact_facts_with_partial_rules_and_unchanged_structure() {
    let f = Fixture::new();
    let (base, mapping) = f.checked();
    let out = f.compile().unwrap();
    assert_eq!(out.receipt.converted_classes, 8);
    assert_eq!(out.receipt.refined_classes, 8);
    assert_eq!(out.receipt.changed_program_owners, 8);
    assert_eq!(out.successor.registry, f.recipe.registry);
    assert_eq!(out.successor.schema.slots, base.schema().input().slots);
    assert_eq!(out.successor.rules.receivers, f.recipe.rules.receivers);
    assert_eq!(
        out.successor.rules.operations_version,
        f.recipe.rules.operations_version
    );
    for before in &base.schema().input().definitions {
        let after = out
            .successor
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == before.address())
            .unwrap();
        if let (DefinitionDescriptor::Class(b), DefinitionDescriptor::Class(a)) = (before, after) {
            let (SchemaState::Known(b), SchemaState::Known(a)) = (&b.schema, &a.schema) else {
                unreachable!()
            };
            assert_eq!(a.level, b.level);
            assert_eq!(a.ascendancies, b.ascendancies);
            assert_eq!(a.implicit_passives, b.implicit_passives);
            assert!(a.declarations.parameters.is_complete());
            assert!(a.declarations.choices.is_complete());
            assert!(a.declarations.grants.is_complete());
            assert!(a.declarations.actors.is_complete());
            assert!(a.declarations.skill_grants.is_complete());
            assert!(a.declarations.outputs.is_complete());
            assert!(a.declarations.sockets.is_complete());
        } else {
            assert_eq!(after, before);
        }
    }
    let checked = assemble_owned_recipe(out.successor, Default::default()).unwrap();
    let compiled = CompiledRulePackage::compile(
        checked.rules().input(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let mut scratch = compiled.new_scratch();
    // Expected values are an independent reviewed source reference, S/D/I.
    for (source_id, values) in [
        (2, [7, 15, 7]),
        (8, [7, 15, 7]),
        (6, [15, 7, 7]),
        (9, [11, 11, 7]),
        (11, [11, 7, 11]),
        (1, [7, 7, 15]),
        (7, [7, 7, 15]),
        (10, [7, 11, 11]),
    ] {
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::Class {
            key: SourceComponent::Text(source_id.to_string()),
        });
        let Some(MappingOutcome::Mapped { target: owner, .. }) = mapping.lookup(&selector) else {
            unreachable!()
        };
        let prior = base
            .rules()
            .input()
            .owners
            .iter()
            .find(|r| r.owner == *owner)
            .unwrap();
        let current = checked
            .rules()
            .input()
            .owners
            .iter()
            .find(|r| r.owner == *owner)
            .unwrap();
        assert_eq!(prior.programs.closure, current.programs.closure);
        assert!(!current.programs.is_complete());
        let p = current
            .programs
            .members
            .iter()
            .find(|p| p.id == key("class-base-contributions"))
            .unwrap();
        let result = compiled
            .evaluate(owner, &p.id, &[], checked.schema(), &mut scratch)
            .unwrap();
        assert_eq!(result.effects.len(), 3);
        for (field, expected) in f.policy.fields.iter().zip(values) {
            let effect = result.effects.iter().find(|e| matches!(&e.effect, RuleEffectKind::Contribute { stat, .. } if stat == &field.stat)).unwrap();
            assert!(matches!(
                &effect.effect,
                RuleEffectKind::Contribute {
                    entity: RuleEntity::Player,
                    contribution: ContributionKind::Add,
                    ..
                }
            ));
            assert_eq!(
                effect.disposition,
                EffectDisposition::Applied {
                    value: ParameterValue::Integer(BoundedInteger::new(expected).unwrap())
                }
            );
        }
    }
}

#[test]
fn arbitrary_pinned_integer_values_and_field_assignments_are_data_driven() {
    let mut f = Fixture::new();
    f.edit_source(|source| {
        for (i, row) in source["classes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .enumerate()
        {
            row["base_str"] = json!(101 + i as i64);
            row["base_dex"] = json!(-17 - i as i64);
            row["base_int"] = json!(0);
        }
    });
    // Alter which channel each field targets; neither names nor IDs infer it.
    f.policy.fields[0].source_field = "base_dex".into();
    f.policy.fields[1].source_field = "base_int".into();
    f.policy.fields[2].source_field = "base_str".into();
    let out = f.compile().unwrap();
    let values: BTreeSet<_> = out
        .successor
        .rules
        .owners
        .iter()
        .filter(|r| {
            matches!(
                r.owner,
                SchemaSubject::Definition(DefinitionAddress::Class(_))
            )
        })
        .flat_map(|r| &r.programs.members)
        .flat_map(|p| &p.nodes)
        .filter_map(|n| {
            if let RuleExpression::Literal {
                value: ParameterValue::Integer(v),
            } = n.expression
            {
                Some(v.get())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(values, (101..109).chain(-24..-16).chain([0]).collect());
}

#[test]
fn idempotent_rerun_preserves_unrelated_programs_and_policy_order_does_not_change_rules() {
    let mut f = Fixture::new();
    let unrelated = RuleProgram {
        id: key("other-class-mechanic"),
        context: RuleEntityKind::Actor,
        reads: vec![],
        nodes: vec![RuleNode {
            id: key("truth"),
            expression: RuleExpression::Literal {
                value: ParameterValue::Boolean(true),
            },
        }],
        effects: vec![],
    };
    let owner = f
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|r| {
            matches!(
                r.owner,
                SchemaSubject::Definition(DefinitionAddress::Class(_))
            )
        })
        .unwrap();
    owner.programs.members.push(unrelated.clone());
    let address = owner.owner.clone();
    let first = f.compile().unwrap();
    f.recipe = first.successor.clone();
    f.policy.fields.reverse();
    f.policy.expected_class_fields.reverse();
    let second = f.compile().unwrap();
    assert!(second.refined.is_empty());
    assert_eq!(second.receipt.changed_program_owners, 0);
    assert_eq!(second.successor, first.successor);
    assert!(
        second
            .successor
            .rules
            .owners
            .iter()
            .find(|r| r.owner == address)
            .unwrap()
            .programs
            .members
            .contains(&unrelated)
    );
}

#[test]
fn exact_source_hash_and_mapping_identity_are_required_and_crlf_is_equivalent() {
    let mut f = Fixture::new();
    f.source.push(b' ');
    bad(&f, "binding differs");
    let mut f = Fixture::new();
    f.policy.source_path = "unregistered.json".into();
    bad(&f, "binding differs");
    let f = Fixture::new();
    let (base, mapping) = f.checked();
    let mut other = f.recipe.clone();
    other.schema.semantics_version = key("different-schema");
    rebind(&mut other);
    let other = assemble_owned_recipe(other, Default::default()).unwrap();
    assert!(matches!(
        compile_owned_class_bases(&other, &mapping, &f.source, &f.policy, Default::default()),
        Err(ClassBaseRecipeError::Binding)
    ));
    let mut f = Fixture::new();
    let v: Value = serde_json::from_slice(&f.source).unwrap();
    f.source = serde_json::to_vec_pretty(&v).unwrap();
    f.repin();
    let lf = f.compile().unwrap();
    f.source = String::from_utf8(f.source)
        .unwrap()
        .replace('\n', "\r\n")
        .into_bytes();
    let crlf = f.compile().unwrap();
    assert_eq!(lf.successor, crlf.successor);
    assert_eq!(lf.receipt.source_sha256, crlf.receipt.source_sha256);
    assert_eq!(base.registry().input(), other.registry().input());
}

#[test]
fn changed_row_shape_membership_and_invalid_scalar_values_are_rejected() {
    for change in [
        |v: &mut Value| {
            v["classes"][0]["new_mechanic"] = json!(3);
        },
        |v: &mut Value| {
            v["classes"].as_array_mut().unwrap().pop();
        },
        |v: &mut Value| {
            let duplicate = v["classes"][0].clone();
            v["classes"].as_array_mut().unwrap().push(duplicate);
        },
        |v: &mut Value| {
            v["classes"][0]["integerId"] = json!(999);
        },
        |v: &mut Value| {
            v["classes"][0]["base_str"] = json!(true);
        },
        |v: &mut Value| {
            v["classes"][0]["base_str"] = json!(3.5);
        },
        |v: &mut Value| {
            v["classes"][0]["base_str"] = json!(BoundedInteger::MAX + 1);
        },
    ] {
        let mut f = Fixture::new();
        f.edit_source(change);
        assert!(f.compile().is_err());
    }
    let mut f = Fixture::new();
    let text = String::from_utf8(f.source).unwrap();
    f.source = text
        .replacen("\"base_str\":", "\"base_str\":9,\"base_str\":", 1)
        .into_bytes();
    f.repin();
    bad(&f, "duplicate class field");
}

#[test]
fn conflicting_existing_rules_complete_membership_and_foreign_port_gaps_reject() {
    let mut f = Fixture::new();
    f.recipe = f.compile().unwrap().successor;
    f.edit_source(|v| {
        v["classes"][0]["base_str"] = json!(123);
    });
    bad(&f, "cannot replace");
    let mut f = Fixture::new();
    f.recipe
        .rules
        .owners
        .iter_mut()
        .find(|r| {
            matches!(
                r.owner,
                SchemaSubject::Definition(DefinitionAddress::Class(_))
            )
        })
        .unwrap()
        .programs
        .closure = SchemaClosure::Complete;
    bad(&f, "cannot replace");
    let mut f = Fixture::new();
    let entry = f
        .recipe
        .schema
        .definitions
        .iter_mut()
        .find_map(|d| {
            if let DefinitionDescriptor::Class(e) = d {
                Some(e)
            } else {
                None
            }
        })
        .unwrap();
    let SchemaState::Known(schema) = &mut entry.schema else {
        unreachable!()
    };
    let SchemaClosure::Partial { gaps } = &mut schema.declarations.actors.closure else {
        unreachable!()
    };
    gaps[0].code = key("unreviewed-class-actor-mechanic");
    rebind(&mut f.recipe);
    bad(&f, "cannot replace");
}

#[test]
fn wrong_stat_scope_type_and_duplicate_policy_fields_reject() {
    let mut f = Fixture::new();
    let id = f.policy.fields[0].stat.clone();
    let DefinitionDescriptor::Stat(entry) = f
        .recipe
        .schema
        .definitions
        .iter_mut()
        .find(|d| d.address() == id.address())
        .unwrap()
    else {
        unreachable!()
    };
    let SchemaState::Known(schema) = &mut entry.schema else {
        unreachable!()
    };
    schema.value = ComputedValueType::Boolean;
    rebind(&mut f.recipe);
    bad(&f, "stat type or scope");
    let mut f = Fixture::new();
    f.policy.fields.push(f.policy.fields[0].clone());
    bad(&f, "duplicate");
    let mut f = Fixture::new();
    f.policy.expected_class_fields.push("base_str".into());
    bad(&f, "duplicate");
    let mut f = Fixture::new();
    f.policy.fields[0].source_field = "integerId".into();
    bad(&f, "identity contribution field");
}

#[test]
fn all_limits_reject_before_mutating_inputs() {
    let f = Fixture::new();
    let (base, mapping) = f.checked();
    for limits in [
        ClassBaseRecipeLimits {
            max_source_bytes: 1,
            ..Default::default()
        },
        ClassBaseRecipeLimits {
            max_policy_bytes: 1,
            ..Default::default()
        },
        ClassBaseRecipeLimits {
            max_classes: 1,
            ..Default::default()
        },
        ClassBaseRecipeLimits {
            max_fields: 1,
            ..Default::default()
        },
        ClassBaseRecipeLimits {
            max_work: 1,
            ..Default::default()
        },
        ClassBaseRecipeLimits {
            max_work: 0,
            ..Default::default()
        },
    ] {
        assert!(compile_owned_class_bases(&base, &mapping, &f.source, &f.policy, limits).is_err());
    }
    assert_eq!(base.schema().input(), &f.recipe.schema);
    assert_eq!(base.rules().input(), &f.recipe.rules);
}
