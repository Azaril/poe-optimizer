//! Finite catalog contract tests; source acquisition/parity has its own boundary.
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition};
use poe_optimizer_import::{owned_intrinsic_attack::*, owned_mapping::*, owned_recipe::*};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf};

fn key(v: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(v).unwrap()
}
fn load<T: DeserializeOwned>(path: &str) -> T {
    serde_json::from_slice(
        &fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../data/owned/poe2/3887ae68/current")
                .join(path),
        )
        .unwrap(),
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
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
struct Fixture {
    recipe: OwnedRecipeInput,
    mapping: MappingPackageInput,
    catalog: IntrinsicAttackCatalog,
    policy: IntrinsicAttackPolicy,
}
impl Fixture {
    fn new() -> Self {
        let mut recipe: OwnedRecipeInput = load("recipe.json");
        let mapping: MappingPackageInput = load("mapping.json");
        let mut registry =
            OwnedIdRegistry::new(recipe.registry.clone(), Default::default()).unwrap();
        let mut unit = |dimension| {
            let id: UnitDefId = registry.allocate_definition().unwrap();
            recipe
                .schema
                .definitions
                .push(DefinitionDescriptor::Unit(DefinitionEntry {
                    id: id.clone(),
                    schema: SchemaState::Known(UnitSchema { dimension }),
                }));
            id
        };
        let rate = unit(UnitDimension::Rate);
        let damage = unit(UnitDimension::Damage);
        let percentage = recipe
            .schema
            .definitions
            .iter()
            .find_map(|d| match d {
                DefinitionDescriptor::Unit(e)
                    if matches!(
                        e.schema,
                        SchemaState::Known(UnitSchema {
                            dimension: UnitDimension::PercentagePoints
                        })
                    ) =>
                {
                    Some(e.id.clone())
                }
                _ => None,
            })
            .unwrap();
        let fields = [
            ("AttackRate", rate),
            ("CritChance", percentage),
            ("PhysicalMin", damage.clone()),
            ("PhysicalMax", damage),
        ]
        .into_iter()
        .map(|(name, unit)| {
            let stat: StatDefId = registry.allocate_definition().unwrap();
            recipe
                .schema
                .definitions
                .push(DefinitionDescriptor::Stat(DefinitionEntry {
                    id: stat.clone(),
                    schema: SchemaState::Known(StatSchema {
                        value: ComputedValueType::Quantity { unit: unit.clone() },
                        targets: vec![RuleEntityKind::Actor],
                    }),
                }));
            IntrinsicAttackField {
                source_field: name.into(),
                stat,
                unit,
            }
        })
        .collect();
        recipe.registry = registry.input().clone();
        rebind(&mut recipe);
        let mut source = mapping.source.clone();
        source
            .files
            .retain(|p| ["src/Modules/Data.lua", "src/Data/Misc.lua"].contains(&p.path.as_str()));
        assert_eq!(source.files.len(), 2);
        // Independent reviewed facts. Optional-PoB exporter tests verify actual
        // source evaluation separately; these tests do not pretend to execute it.
        let classes = [
            (0, 6.0),
            (1, 5.0),
            (2, 5.0),
            (6, 8.0),
            (7, 5.0),
            (8, 5.0),
            (9, 6.0),
            (10, 5.0),
            (11, 6.0),
        ]
        .into_iter()
        .map(|(id, max)| IntrinsicAttackClass {
            class_key: id.to_string(),
            fields: BTreeMap::from([
                ("type".into(), IntrinsicSourceValue::Text("None".into())),
                ("AttackRate".into(), IntrinsicSourceValue::Number(1.65)),
                ("CritChance".into(), IntrinsicSourceValue::Number(5.0)),
                ("PhysicalMin".into(), IntrinsicSourceValue::Number(2.0)),
                ("PhysicalMax".into(), IntrinsicSourceValue::Number(max)),
            ]),
        })
        .collect();
        let mut f = Self {
            recipe,
            mapping,
            catalog: IntrinsicAttackCatalog {
                schema_version: OWNED_INTRINSIC_ATTACK_VERSION,
                source,
                classes,
            },
            policy: IntrinsicAttackPolicy {
                schema_version: OWNED_INTRINSIC_ATTACK_VERSION,
                version: key("reviewed-intrinsic-facts"),
                catalog_sha256: String::new(),
                excluded_classes: vec!["0".into()],
                constants: BTreeMap::from([(
                    "type".into(),
                    IntrinsicSourceValue::Text("None".into()),
                )]),
                fields,
            },
        };
        f.repin();
        f
    }
    fn wire(&self) -> Vec<u8> {
        let mut bytes = serde_json::to_vec(&self.catalog).unwrap();
        bytes.push(b'\n');
        bytes
    }
    fn repin(&mut self) {
        self.policy.catalog_sha256 = hash(&self.wire());
    }
    fn checked(&self) -> (StagedOwnedRecipe, OwnedMappingIndex) {
        let base = assemble_owned_recipe(self.recipe.clone(), Default::default()).unwrap();
        let mut mapping = self.mapping.clone();
        mapping.registry = base.registry().identity().unwrap();
        mapping.definitions = base.schema().identity().clone();
        let mapping =
            OwnedMappingIndex::new(mapping, base.registry(), base.schema(), Default::default())
                .unwrap();
        (base, mapping)
    }
    fn compile(&self) -> Result<StagedIntrinsicAttackRecipe, IntrinsicAttackError> {
        let (base, mapping) = self.checked();
        compile_owned_intrinsic_attack(
            &base,
            &mapping,
            &self.wire(),
            &self.policy,
            Default::default(),
        )
    }
}
fn bad(f: &Fixture, needle: &str) {
    let error = f.compile().unwrap_err();
    assert!(error.to_string().contains(needle), "{error}");
}
fn first_class(input: &mut OwnedRecipeInput) -> &mut DefinitionRules {
    input
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
}
fn outputs(
    f: &Fixture,
    out: &OwnedRecipeInput,
) -> BTreeMap<String, BTreeMap<StatDefId, FiniteQuantity>> {
    let checked = assemble_owned_recipe(out.clone(), Default::default()).unwrap();
    let compiled = CompiledRulePackage::compile(
        checked.rules().input(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let (_, mapping) = f.checked();
    let mut scratch = compiled.new_scratch();
    let mut all = BTreeMap::new();
    for row in &f.catalog.classes {
        if f.policy.excluded_classes.contains(&row.class_key) {
            continue;
        }
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::Class {
            key: SourceComponent::Text(row.class_key.clone()),
        });
        let Some(MappingOutcome::Mapped { target: owner, .. }) = mapping.lookup(&selector) else {
            unreachable!()
        };
        let result = compiled
            .evaluate(
                owner,
                &key("intrinsic-attack-baseline"),
                &[],
                checked.schema(),
                &mut scratch,
            )
            .unwrap();
        let values = result
            .effects
            .iter()
            .map(|e| {
                let RuleEffectKind::Derive {
                    entity: RuleEntity::Player,
                    stat,
                    ..
                } = &e.effect
                else {
                    panic!("exclusive player baseline")
                };
                let EffectDisposition::Applied {
                    value: ParameterValue::Quantity(value),
                } = &e.disposition
                else {
                    panic!("known quantity")
                };
                (stat.clone(), value.clone())
            })
            .collect();
        all.insert(row.class_key.clone(), values);
    }
    all
}

#[test]
fn all_eight_class_baselines_are_typed_derivations_without_any_other_state_change() {
    let f = Fixture::new();
    let (base, _) = f.checked();
    let out = f.compile().unwrap();
    assert_eq!(out.receipt.converted_classes, 8);
    assert_eq!(out.receipt.excluded_classes, 1);
    assert_eq!(out.receipt.changed_program_owners, 8);
    assert_eq!(
        out.receipt.before_definitions,
        out.receipt.after_definitions
    );
    assert_eq!(out.successor.registry, *base.registry().input());
    assert_eq!(out.successor.schema, *base.schema().input());
    assert_eq!(out.successor.routing, *base.routing().input());
    assert_eq!(
        out.successor.rules.receivers,
        base.rules().input().receivers
    );
    assert_eq!(
        out.successor.rules.operations_version,
        base.rules().input().operations_version
    );
    for old in &base.rules().input().owners {
        let current = out
            .successor
            .rules
            .owners
            .iter()
            .find(|r| r.owner == old.owner)
            .unwrap();
        assert_eq!(current.programs.closure, old.programs.closure);
        if matches!(
            old.owner,
            SchemaSubject::Definition(DefinitionAddress::Class(_))
        ) {
            assert!(!current.programs.is_complete());
            assert_eq!(
                current.programs.members.len(),
                old.programs.members.len() + 1
            );
        } else {
            assert_eq!(current, old);
        }
    }
    for (class, values) in outputs(&f, &out.successor) {
        assert_eq!(values.len(), 4);
        let max = match class.as_str() {
            "6" => 8.0,
            "9" | "11" => 6.0,
            _ => 5.0,
        };
        for field in &f.policy.fields {
            let expected = match field.source_field.as_str() {
                "AttackRate" => 1.65,
                "CritChance" => 5.0,
                "PhysicalMin" => 2.0,
                "PhysicalMax" => max,
                _ => unreachable!(),
            };
            assert_eq!(
                values[&field.stat],
                FiniteQuantity::new(expected, field.unit.clone()).unwrap()
            );
        }
    }
}

#[test]
fn values_units_and_field_bindings_are_injected_even_when_names_or_coefficients_change() {
    let mut f = Fixture::new();
    // Rename a source field and swap its channel binding with a compatible one.
    for row in &mut f.catalog.classes {
        let value = row.fields.remove("PhysicalMin").unwrap();
        row.fields.insert("CustomMinimum".into(), value);
        row.fields
            .insert("AttackRate".into(), IntrinsicSourceValue::Number(2.125));
        row.fields
            .insert("CustomMinimum".into(), IntrinsicSourceValue::Number(-11.25));
        row.fields
            .insert("PhysicalMax".into(), IntrinsicSourceValue::Number(93.75));
    }
    f.policy.fields[2].source_field = "PhysicalMax".into();
    f.policy.fields[3].source_field = "CustomMinimum".into();
    f.repin();
    let out = f.compile().unwrap();
    for values in outputs(&f, &out.successor).values() {
        assert_eq!(values[&f.policy.fields[0].stat].value(), 2.125);
        assert_eq!(values[&f.policy.fields[2].stat].value(), 93.75);
        assert_eq!(values[&f.policy.fields[3].stat].value(), -11.25);
        for field in &f.policy.fields {
            assert_eq!(values[&field.stat].unit(), &field.unit);
        }
    }
}

#[test]
fn exact_reruns_are_idempotent_and_keep_unrelated_prior_programs() {
    let mut f = Fixture::new();
    let prior = RuleProgram {
        id: key("other-class-rule"),
        context: RuleEntityKind::Actor,
        reads: vec![],
        nodes: vec![RuleNode {
            id: key("yes"),
            expression: RuleExpression::Literal {
                value: ParameterValue::Boolean(true),
            },
        }],
        effects: vec![],
    };
    first_class(&mut f.recipe)
        .programs
        .members
        .push(prior.clone());
    let first = f.compile().unwrap();
    f.recipe = first.successor.clone();
    f.policy.fields.reverse();
    f.catalog.classes.reverse();
    f.repin();
    let again = f.compile().unwrap();
    assert_eq!(again.receipt.changed_program_owners, 0);
    assert_eq!(again.successor, first.successor);
    assert!(
        again
            .successor
            .rules
            .owners
            .iter()
            .any(|r| r.programs.members.contains(&prior))
    );
}

#[test]
fn artifact_authority_is_explicit_and_distinct_from_carried_source_provenance() {
    let mut f = Fixture::new();
    f.catalog.classes[1]
        .fields
        .insert("AttackRate".into(), IntrinsicSourceValue::Number(123.0));
    bad(&f, "binding differs");
    // A caller can explicitly approve different data bytes. Carried source pins
    // alone never purport to prove source evaluation inside this pure compiler.
    f.repin();
    assert!(f.compile().is_ok());
    f.catalog.source.files[0].sha256 = "0".repeat(64);
    f.repin();
    bad(&f, "binding differs");
    let mut f = Fixture::new();
    f.catalog
        .source
        .files
        .push(f.catalog.source.files[0].clone());
    f.repin();
    bad(&f, "binding differs");
    let f = Fixture::new();
    let (base, mapping) = f.checked();
    let mut extra = f.wire();
    extra.push(b' ');
    assert!(matches!(
        compile_owned_intrinsic_attack(&base, &mapping, &extra, &f.policy, Default::default()),
        Err(IntrinsicAttackError::Binding)
    ));
}

#[test]
fn all_source_classes_and_explicit_exclusions_must_match_the_owned_family() {
    let mut f = Fixture::new();
    f.catalog.classes.pop();
    f.repin();
    bad(&f, "membership differs");
    let mut f = Fixture::new();
    f.catalog.classes.push(f.catalog.classes[1].clone());
    f.repin();
    bad(&f, "duplicate or invalid source class");
    let mut f = Fixture::new();
    f.catalog.classes[1].class_key = "999".into();
    f.repin();
    bad(&f, "unresolved class selector");
    let mut f = Fixture::new();
    f.policy.excluded_classes.clear();
    bad(&f, "unresolved class selector");
    let mut f = Fixture::new();
    f.policy.excluded_classes.push("1".into());
    bad(&f, "mapped excluded class");
    let mut f = Fixture::new();
    f.policy.excluded_classes.push("999".into());
    bad(&f, "excluded class missing");
    let mut f = Fixture::new();
    f.catalog.classes[0].class_key = "00".into();
    f.repin();
    bad(&f, "duplicate or invalid source class");
}

#[test]
fn field_shape_constants_numeric_types_and_duplicate_json_keys_reject() {
    let mut f = Fixture::new();
    f.catalog.classes[1]
        .fields
        .insert("unreviewed".into(), IntrinsicSourceValue::Number(1.0));
    f.repin();
    bad(&f, "field membership");
    let mut f = Fixture::new();
    f.catalog.classes[0].fields.insert(
        "type".into(),
        IntrinsicSourceValue::Text("Different".into()),
    );
    f.repin();
    bad(&f, "field membership");
    let mut f = Fixture::new();
    f.catalog.classes[1].fields.insert(
        "AttackRate".into(),
        IntrinsicSourceValue::Text("1.65".into()),
    );
    f.repin();
    bad(&f, "numeric output");
    let f = Fixture::new();
    let (base, mapping) = f.checked();
    for replacement in [
        r#""type":"None","type":"None""#,
        r#""type":null"#,
        r#""type":true"#,
    ] {
        let text =
            String::from_utf8(f.wire())
                .unwrap()
                .replacen(r#""type":"None""#, replacement, 1);
        let mut policy = f.policy.clone();
        policy.catalog_sha256 = hash(text.as_bytes());
        assert!(
            compile_owned_intrinsic_attack(
                &base,
                &mapping,
                text.as_bytes(),
                &policy,
                Default::default()
            )
            .is_err()
        );
    }
    let text = String::from_utf8(f.wire()).unwrap().replacen(
        r#""schema_version":1"#,
        r#""schema_version":1,"schema_version":1"#,
        1,
    );
    let mut policy = f.policy.clone();
    policy.catalog_sha256 = hash(text.as_bytes());
    assert!(
        compile_owned_intrinsic_attack(
            &base,
            &mapping,
            text.as_bytes(),
            &policy,
            Default::default()
        )
        .is_err()
    );
}

#[test]
fn conflicting_exclusive_writers_and_closed_owner_membership_cannot_be_replaced() {
    let mut f = Fixture::new();
    f.recipe = f.compile().unwrap().successor;
    f.catalog.classes[1]
        .fields
        .insert("PhysicalMax".into(), IntrinsicSourceValue::Number(99.0));
    f.repin();
    bad(&f, "exclusive stat writers");
    for contribution in [false, true] {
        let mut f = Fixture::new();
        let field = &f.policy.fields[0];
        let effect = if contribution {
            RuleEffectKind::Contribute {
                entity: RuleEntity::Player,
                stat: field.stat.clone(),
                contribution: ContributionKind::Add,
                value: key("value"),
            }
        } else {
            RuleEffectKind::Derive {
                entity: RuleEntity::Player,
                stat: field.stat.clone(),
                value: key("value"),
            }
        };
        let p = RuleProgram {
            id: key("unrelated-writer"),
            context: RuleEntityKind::Actor,
            reads: vec![],
            nodes: vec![RuleNode {
                id: key("value"),
                expression: RuleExpression::Literal {
                    value: ParameterValue::Quantity(
                        FiniteQuantity::new(8.0, field.unit.clone()).unwrap(),
                    ),
                },
            }],
            effects: vec![RuleEffect {
                id: key("write"),
                when: None,
                effect,
            }],
        };
        first_class(&mut f.recipe).programs.members.push(p);
        bad(&f, "exclusive stat writers");
    }
    let mut f = Fixture::new();
    first_class(&mut f.recipe).programs.closure = SchemaClosure::Complete;
    bad(&f, "exclusive stat writers");
}

#[test]
fn injected_output_units_scopes_and_duplicate_bindings_are_checked() {
    let mut f = Fixture::new();
    f.policy.fields[0].unit = f.policy.fields[1].unit.clone();
    bad(&f, "type, unit or scope");
    let mut f = Fixture::new();
    f.policy.fields.push(f.policy.fields[0].clone());
    bad(&f, "duplicate output field");
    let mut f = Fixture::new();
    f.policy
        .constants
        .insert("AttackRate".into(), IntrinsicSourceValue::Number(1.65));
    bad(&f, "duplicate output field");
    let mut f = Fixture::new();
    let stat = f.policy.fields[0].stat.clone();
    let DefinitionDescriptor::Stat(e) = f
        .recipe
        .schema
        .definitions
        .iter_mut()
        .find(|d| d.address() == stat.address())
        .unwrap()
    else {
        unreachable!()
    };
    let SchemaState::Known(s) = &mut e.schema else {
        unreachable!()
    };
    s.targets = vec![RuleEntityKind::Action];
    rebind(&mut f.recipe);
    bad(&f, "type, unit or scope");
}

#[test]
fn resource_limits_and_foreign_predecessor_identity_fail_without_mutating_inputs() {
    let f = Fixture::new();
    let (base, mapping) = f.checked();
    for limits in [
        IntrinsicAttackLimits {
            max_catalog_bytes: 1,
            ..Default::default()
        },
        IntrinsicAttackLimits {
            max_policy_bytes: 1,
            ..Default::default()
        },
        IntrinsicAttackLimits {
            max_classes: 1,
            ..Default::default()
        },
        IntrinsicAttackLimits {
            max_fields: 1,
            ..Default::default()
        },
        IntrinsicAttackLimits {
            max_work: 1,
            ..Default::default()
        },
        IntrinsicAttackLimits {
            max_work: 0,
            ..Default::default()
        },
    ] {
        assert!(
            compile_owned_intrinsic_attack(&base, &mapping, &f.wire(), &f.policy, limits).is_err()
        );
    }
    let mut other = f.recipe.clone();
    other.schema.release = key("foreign-release");
    rebind(&mut other);
    let other = assemble_owned_recipe(other, Default::default()).unwrap();
    assert!(matches!(
        compile_owned_intrinsic_attack(&other, &mapping, &f.wire(), &f.policy, Default::default()),
        Err(IntrinsicAttackError::Binding)
    ));
    assert_eq!(
        base.rules().input().owners.len(),
        f.recipe.rules.owners.len()
    );
}
