//! Complete finite weapon-profile laws consume the maintained KDL/DSL release.
//! Raw channels and source presence do not establish final attacks or complete items.
use super::tracked_release;
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_engine::owned_rules::EffectDisposition;
use poe_optimizer_import::owned_mapping::{
    ExternalOwnerSelector, ExternalSelector, ExternalSourceSystem, MappingBasis, MappingOutcome,
    SourceComponent, SourcePin,
};
use serde::{Deserialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    sync::OnceLock,
};

// These private evidence projections do not retain the historical compiler API.
#[derive(Deserialize)]
struct Catalog {
    source: SourcePin,
    base_catalog_sha256: String,
    profiles: Vec<Profile>,
}
#[derive(Deserialize)]
struct Profile {
    base: String,
    fields: BTreeMap<String, f64>,
}
#[derive(Deserialize)]
struct Policy {
    catalog_sha256: String,
    base_catalog_sha256: String,
    fields: Vec<Field>,
}
#[derive(Deserialize)]
struct Field {
    source_field: String,
    stat: StatDefId,
    unit: UnitDefId,
    when_absent: Absence,
    presence: Option<CapabilityDefId>,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Absence {
    Required,
    Literal { value: ParameterValue },
    Omit,
}
#[derive(Deserialize)]
struct Bases {
    source: SourcePin,
    bases: Vec<Base>,
}
#[derive(Deserialize)]
struct Base {
    name: String,
    weapon_field: WeaponField,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum WeaponField {
    Table,
    Absent,
}
#[derive(Deserialize)]
struct Templates {
    catalog_sha256: String,
    templates: Vec<Template>,
}
#[derive(Deserialize)]
struct Template {
    source_base: String,
    template: ItemTemplateDefId,
}
struct Evidence {
    catalog: Catalog,
    policy: Policy,
    bases: Bases,
    templates: Templates,
}
fn pinned<T: DeserializeOwned>(path: &str, digest: &str) -> T {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/owned/poe2/3887ae68")
        .join(path);
    let bytes = fs::read(path).unwrap();
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), digest);
    serde_json::from_slice(&bytes).unwrap()
}
fn evidence() -> &'static Evidence {
    static EVIDENCE: OnceLock<Evidence> = OnceLock::new();
    EVIDENCE.get_or_init(|| {
        let catalog: Catalog = pinned(
            "weapon-profiles/catalog.json",
            "1ed33405c5862b9705d7ceff238cc516246943785c3894b8923ae8f8df7db3a6",
        );
        let policy: Policy = pinned(
            "weapon-profiles/policy.json",
            "6446a4d464af485e716822879ddd0008b621cd9ed2adaf2a9f46a434090cce1f",
        );
        let bases: Bases = pinned(
            "item-bases/catalog.json",
            "6dda4b5aa9371050b72343542219ad54d6b469ce16af7ede3c0e9cacbbec9554",
        );
        let templates: Templates = pinned(
            "item-bases/policy.json",
            "8a2880b1ce37c6d89396436f90b1a9b7897dba75ae1b161044a617142a506e81",
        );
        assert_eq!(
            policy.catalog_sha256,
            "1ed33405c5862b9705d7ceff238cc516246943785c3894b8923ae8f8df7db3a6"
        );
        assert_eq!(catalog.base_catalog_sha256, templates.catalog_sha256);
        assert_eq!(policy.base_catalog_sha256, templates.catalog_sha256);
        assert_eq!(
            templates.catalog_sha256,
            "6dda4b5aa9371050b72343542219ad54d6b469ce16af7ede3c0e9cacbbec9554"
        );
        assert_eq!(catalog.source, bases.source);
        assert_eq!(catalog.source.system, ExternalSourceSystem::PathOfBuilding2);
        assert_eq!(
            catalog.source.revision,
            "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
        );
        assert_eq!(catalog.source.files.len(), 29);
        assert_eq!(catalog.profiles.len(), 337);
        assert_eq!(bases.bases.len(), 1756);
        assert_eq!(templates.templates.len(), 1756);
        Evidence {
            catalog,
            policy,
            bases,
            templates,
        }
    })
}
fn selector(name: &str) -> ExternalSelector {
    ExternalSelector::Definition(ExternalOwnerSelector::ItemTemplate {
        base: SourceComponent::Text(name.into()),
        prototype: SourceComponent::Missing,
        variant: SourceComponent::Missing,
    })
}
fn quantity(value: f64, unit: &UnitDefId) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(value, unit.clone()).unwrap())
}
fn expected_value(profile: &Profile, field: &Field) -> Option<ParameterValue> {
    profile
        .fields
        .get(&field.source_field)
        .map(|value| quantity(*value, &field.unit))
        .or_else(|| match &field.when_absent {
            Absence::Required => panic!(
                "required source field {} in {}",
                field.source_field, profile.base
            ),
            Absence::Literal { value } => Some(value.clone()),
            Absence::Omit => None,
        })
}

#[test]
fn whole_catalog_membership_and_exact_template_owners_preserve_partial_coverage() {
    let evidence = evidence();
    let release = tracked_release().release();
    let recipe = release.assembled().checked();
    let bases: BTreeSet<_> = evidence.bases.bases.iter().map(|row| &row.name).collect();
    let bindings: BTreeSet<_> = evidence
        .templates
        .templates
        .iter()
        .map(|row| &row.source_base)
        .collect();
    let profiles: BTreeSet<_> = evidence
        .catalog
        .profiles
        .iter()
        .map(|row| &row.base)
        .collect();
    let tables: BTreeSet<_> = evidence
        .bases
        .bases
        .iter()
        .filter_map(|row| matches!(row.weapon_field, WeaponField::Table).then_some(&row.name))
        .collect();
    assert_eq!(bases.len(), 1756);
    assert_eq!(bases, bindings);
    assert_eq!(profiles.len(), 337);
    assert_eq!(profiles, tables);
    let mut identities = BTreeSet::new();
    let mut counts = [0, 0];
    for binding in &evidence.templates.templates {
        assert!(identities.insert(&binding.template));
        let owner = SchemaSubject::Definition(binding.template.address());
        assert_eq!(
            release.mapping().lookup(&selector(&binding.source_base)),
            Some(&MappingOutcome::Mapped {
                target: owner.clone(),
                basis: MappingBasis::Exact,
            })
        );
        let DefinitionDescriptor::ItemTemplate(descriptor) = recipe
            .schema()
            .lookup_definition(&binding.template.address())
            .unwrap()
        else {
            panic!("typed ItemTemplate")
        };
        assert_eq!(descriptor.id, binding.template);
        assert!(matches!(descriptor.schema, SchemaState::Known(_)));
        let rows: Vec<_> = recipe
            .rules()
            .input()
            .owners
            .iter()
            .filter(|row| row.owner == owner)
            .collect();
        let [row] = rows.as_slice() else {
            panic!("one exact rule owner")
        };
        let SchemaClosure::Partial { gaps } = &row.programs.closure else {
            panic!("raw profile must not close item mechanics")
        };
        assert!(!gaps.is_empty());
        assert!(gaps.iter().all(|gap| gap.subject == owner));
        let programs: Vec<_> = row
            .programs
            .members
            .iter()
            .filter(|program| program.id.as_str() == "raw-base-attack-profile")
            .collect();
        let has_profile = profiles.contains(&binding.source_base);
        assert_eq!(
            programs.len(),
            usize::from(has_profile),
            "{}",
            binding.source_base
        );
        // Absence is not an all-zero profile or a false reload fact: no numeric
        // program, and hence neither numeric outputs nor reload presence exists.
        counts[usize::from(has_profile)] += 1;
    }
    assert_eq!(counts, [1419, 337]);
}

#[test]
fn fourteen_channels_keep_exact_units_defaults_omission_and_authored_zero_counts() {
    let evidence = evidence();
    let schema = tracked_release().release().assembled().checked().schema();
    // Independent field/source counts include values actually authored as zero.
    let expected = [
        ("AttackRateBase", 0x1d39, 337, 0),
        ("ChaosMax", 0x1d3a, 5, 0),
        ("ChaosMin", 0x1d3a, 5, 0),
        ("ColdMax", 0x1d3a, 5, 0),
        ("ColdMin", 0x1d3a, 5, 0),
        ("CritChanceBase", 0x0002, 337, 2),
        ("FireMax", 0x1d3a, 8, 0),
        ("FireMin", 0x1d3a, 8, 0),
        ("LightningMax", 0x1d3a, 10, 0),
        ("LightningMin", 0x1d3a, 10, 1),
        ("PhysicalMax", 0x1d3a, 331, 2),
        ("PhysicalMin", 0x1d3a, 331, 2),
        ("Range", 0x2425, 337, 0),
        ("ReloadTimeBase", 0x0005, 33, 0),
    ];
    assert_eq!(evidence.policy.fields.len(), expected.len());
    let known: BTreeSet<_> = expected.iter().map(|row| row.0).collect();
    let observed: BTreeSet<_> = evidence
        .catalog
        .profiles
        .iter()
        .flat_map(|row| row.fields.keys().map(String::as_str))
        .collect();
    assert_eq!(known, observed);
    for (index, (field, (name, unit, present, zeros))) in
        evidence.policy.fields.iter().zip(expected).enumerate()
    {
        assert_eq!(field.source_field, name);
        assert_eq!(
            field.stat.key().as_str(),
            format!("def.{:016x}", 0x2417 + index)
        );
        assert_eq!(field.unit.key().as_str(), format!("def.{unit:016x}"));
        let DefinitionDescriptor::Stat(stat) =
            schema.lookup_definition(&field.stat.address()).unwrap()
        else {
            panic!("typed Stat")
        };
        let SchemaState::Known(stat_schema) = &stat.schema else {
            panic!("known Stat")
        };
        assert_eq!(stat_schema.targets, [RuleEntityKind::EquipmentUse]);
        assert_eq!(
            stat_schema.value,
            ComputedValueType::Quantity {
                unit: field.unit.clone()
            }
        );
        let DefinitionDescriptor::Unit(unit_descriptor) =
            schema.lookup_definition(&field.unit.address()).unwrap()
        else {
            panic!("typed Unit")
        };
        let SchemaState::Known(unit_schema) = &unit_descriptor.schema else {
            panic!("known Unit")
        };
        let dimension = match name {
            "AttackRateBase" => UnitDimension::Rate,
            "CritChanceBase" => UnitDimension::PercentagePoints,
            "Range" => UnitDimension::Distance,
            "ReloadTimeBase" => UnitDimension::Time,
            _ => UnitDimension::Damage,
        };
        assert_eq!(unit_schema.dimension, dimension);
        match name {
            "AttackRateBase" | "CritChanceBase" | "Range" => {
                assert!(matches!(field.when_absent, Absence::Required))
            }
            "ReloadTimeBase" => assert!(matches!(field.when_absent, Absence::Omit)),
            _ => {
                let Absence::Literal { value } = &field.when_absent else {
                    panic!("explicit damage default")
                };
                assert_eq!(*value, quantity(0.0, &field.unit));
            }
        }
        if name == "ReloadTimeBase" {
            let capability = field.presence.as_ref().unwrap();
            assert_eq!(capability.key().as_str(), "def.0000000000002426");
            let DefinitionDescriptor::Capability(capability) =
                schema.lookup_definition(&capability.address()).unwrap()
            else {
                panic!("typed Capability")
            };
            let SchemaState::Known(capability) = &capability.schema else {
                panic!("known Capability")
            };
            assert_eq!(capability.targets, [RuleEntityKind::EquipmentUse]);
        } else {
            assert!(field.presence.is_none());
        }
        let values: Vec<_> = evidence
            .catalog
            .profiles
            .iter()
            .filter_map(|row| row.fields.get(name))
            .collect();
        assert_eq!(values.len(), present, "{name}");
        assert_eq!(
            values.iter().filter(|value| ***value == 0.0).count(),
            zeros,
            "{name}"
        );
    }
}

#[test]
fn every_profile_executes_all_raw_channels_and_presence_with_fresh_and_reused_scratch() {
    let evidence = evidence();
    let recipe = tracked_release().release().assembled().checked();
    let compiled = recipe.compiled_rules();
    let mut scratch = compiled.new_scratch();
    for reverse in [false, true] {
        let mut profiles: Vec<_> = evidence.catalog.profiles.iter().collect();
        if reverse {
            profiles.reverse();
        }
        // source values, defaults, omissions, presence true, presence false.
        let mut counts = [0usize; 5];
        for profile in profiles {
            let binding = evidence
                .templates
                .templates
                .iter()
                .find(|binding| binding.source_base == profile.base)
                .unwrap();
            let owner = SchemaSubject::Definition(binding.template.address());
            let rules = recipe
                .rules()
                .input()
                .owners
                .iter()
                .find(|row| row.owner == owner)
                .unwrap();
            let program = rules
                .programs
                .members
                .iter()
                .find(|program| program.id.as_str() == "raw-base-attack-profile")
                .unwrap();
            assert_eq!(program.context, RuleEntityKind::EquipmentUse);
            assert!(program.reads.is_empty());
            assert!(program.effects.iter().all(|effect| effect.when.is_none()));
            assert!(
                program
                    .nodes
                    .iter()
                    .all(|node| matches!(node.expression, RuleExpression::Literal { .. }))
            );
            let fresh = compiled
                .evaluate(
                    &owner,
                    &program.id,
                    &[],
                    recipe.schema().as_ref(),
                    &mut compiled.new_scratch(),
                )
                .unwrap();
            let reused = compiled
                .evaluate(
                    &owner,
                    &program.id,
                    &[],
                    recipe.schema().as_ref(),
                    &mut scratch,
                )
                .unwrap();
            assert_eq!(fresh, reused);
            assert_eq!(reused.owner, owner);
            assert_eq!(reused.owner_programs_closure, rules.programs.closure);
            let mut expected_ids = BTreeSet::new();
            for (index, field) in evidence.policy.fields.iter().enumerate() {
                let value = expected_value(profile, field);
                if profile.fields.contains_key(&field.source_field) {
                    counts[0] += 1;
                } else if value.is_some() {
                    counts[1] += 1;
                } else {
                    counts[2] += 1;
                }
                let matching: Vec<_> = reused.effects.iter().filter(|effect| matches!(&effect.effect, RuleEffectKind::Derive { stat, .. } if stat == &field.stat)).collect();
                if let Some(expected) = value {
                    let [actual] = matching.as_slice() else {
                        panic!(
                            "one numeric output for {} {}",
                            profile.base, field.source_field
                        )
                    };
                    let expected_id = format!("raw-{index}");
                    assert_eq!(actual.id.as_str(), expected_id);
                    assert!(expected_ids.insert(expected_id));
                    assert!(matches!(
                        &actual.effect,
                        RuleEffectKind::Derive {
                            entity: RuleEntity::Current,
                            ..
                        }
                    ));
                    assert_eq!(
                        actual.disposition,
                        EffectDisposition::Applied { value: expected }
                    );
                } else {
                    assert!(matching.is_empty(), "omitted reload is not zero");
                }
                if let Some(presence) = &field.presence {
                    let matching: Vec<_> = reused.effects.iter().filter(|effect| matches!(&effect.effect, RuleEffectKind::Capability { capability, .. } if capability == presence)).collect();
                    let [actual] = matching.as_slice() else {
                        panic!("one field-presence output")
                    };
                    let expected_id = format!("field-present-{index}");
                    assert_eq!(actual.id.as_str(), expected_id);
                    assert!(expected_ids.insert(expected_id));
                    assert!(matches!(
                        &actual.effect,
                        RuleEffectKind::Capability {
                            entity: RuleEntity::Current,
                            ..
                        }
                    ));
                    let present = profile.fields.contains_key(&field.source_field);
                    assert_eq!(
                        actual.disposition,
                        EffectDisposition::Applied {
                            value: ParameterValue::Boolean(present)
                        }
                    );
                    counts[if present { 3 } else { 4 }] += 1;
                }
            }
            let actual_ids: BTreeSet<_> = reused
                .effects
                .iter()
                .map(|effect| effect.id.as_str().to_owned())
                .collect();
            assert_eq!(actual_ids, expected_ids);
            assert_eq!(reused.effects.len(), expected_ids.len());
            assert_eq!(program.effects.len(), expected_ids.len());
            assert_eq!(program.nodes.len(), expected_ids.len());
        }
        assert_eq!(counts, [1762, 2652, 304, 33, 304]);
    }
    // The synthetic Import law separately retains present-zero reload behavior;
    // this pinned real catalog has 33 positive reload values and no authored zero.
    // Current originals/checkpoint laws own query preservation and publication;
    // historical all-item/all-allocation Pending expectations are not repeated.
}
