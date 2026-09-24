//! Raw EquipmentUse profiles preserve pending item and whole-build coverage.
use super::{
    passive_attributes::{check_original_preservation, check_rebound_inputs},
    scalar_families::recipe,
    support::{bundle, data, json, success},
};
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_definitions::*,
    owned_rules::{RuleEffectKind, RuleEntity},
    owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition};
use poe_optimizer_import::{
    owned_defence_profiles::{
        DefenceFieldAbsence, DefenceProfileCatalog, DefenceProfilePolicy, DefenceProfilePresence,
    },
    owned_item_bases::ItemBasePolicy,
    owned_mapping::OwnedIdRegistry,
    owned_recipe::{OwnedRecipeInput, assemble_owned_recipe},
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn publish(cwd: &Path, prior: &Path, policy: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .arg("compile-owned-defence-profiles")
        .arg(prior)
        .arg("--base-catalog")
        .arg(data().join("item-bases/catalog.json"))
        .arg("--catalog")
        .arg(data().join("defence-profiles/catalog.json"))
        .arg("--policy")
        .arg(policy)
        .arg("--definitions")
        .arg(data().join("defence-profiles/definitions.json"))
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}

fn check_definitions(
    before: &OwnedRecipeInput,
    after: &OwnedRecipeInput,
    policy: &DefenceProfilePolicy,
) {
    let definitions: Vec<DefinitionDescriptor> =
        serde_json::from_value(json(data().join("defence-profiles/definitions.json"))).unwrap();
    assert_eq!(definitions.len(), 9);
    assert_eq!(before.registry.last_issued.get(), 10_745);
    assert_eq!(after.registry.last_issued.get(), 10_754);
    let mut registry = OwnedIdRegistry::new(before.registry.clone(), Default::default()).unwrap();
    let (mut units, mut stats, mut capabilities) = (0, 0, 0);
    for definition in &definitions {
        match definition {
            DefinitionDescriptor::Unit(entry) => {
                assert!(matches!(entry.schema, SchemaState::Known(_)));
                assert_eq!(
                    registry.allocate_definition::<UnitDefinition>().unwrap(),
                    entry.id
                );
                assert_eq!(entry.id.key().as_str(), "def.00000000000029fa");
                units += 1;
            }
            DefinitionDescriptor::Stat(entry) => {
                assert_eq!(
                    registry.allocate_definition::<StatDefinition>().unwrap(),
                    entry.id
                );
                let SchemaState::Known(schema) = &entry.schema else {
                    panic!("known raw stat")
                };
                let field = policy
                    .fields
                    .iter()
                    .find(|field| field.stat == entry.id)
                    .unwrap();
                assert_eq!(schema.targets, [RuleEntityKind::EquipmentUse]);
                assert_eq!(
                    schema.value,
                    ComputedValueType::Quantity {
                        unit: field.unit.clone()
                    }
                );
                stats += 1;
            }
            DefinitionDescriptor::Capability(entry) => {
                assert_eq!(
                    registry
                        .allocate_definition::<CapabilityDefinition>()
                        .unwrap(),
                    entry.id
                );
                let SchemaState::Known(schema) = &entry.schema else {
                    panic!("known field presence")
                };
                assert_eq!(schema.targets, [RuleEntityKind::EquipmentUse]);
                assert_eq!(
                    policy
                        .fields
                        .iter()
                        .filter(|field| field.presence.as_ref() == Some(&entry.id))
                        .count(),
                    1
                );
                capabilities += 1;
            }
            _ => panic!("raw profile may append only Unit, EquipmentUse Stat/Capability"),
        }
    }
    assert_eq!((units, stats, capabilities), (1, 6, 2));
    assert_eq!(&after.registry, registry.input());
    let mut expected_schema = before.schema.clone();
    expected_schema.definitions.extend(definitions);
    expected_schema
        .definitions
        .sort_by_cached_key(DefinitionDescriptor::address);
    assert_eq!(
        after.schema, expected_schema,
        "existing descriptors and slots must remain exact"
    );
    let mut routing = after.routing.clone();
    routing.definitions = before.routing.definitions.clone();
    assert_eq!(routing, before.routing);
}

pub fn check_defence_profiles(cwd: &Path, prior: &Path) -> PathBuf {
    let authored = data().join("defence-profiles");
    let authored_bytes = bundle(&authored);
    let prior_bytes = bundle(prior);
    let policy_path = authored.join("policy.json");
    let policy: DefenceProfilePolicy = serde_json::from_value(json(&policy_path)).unwrap();
    let catalog: DefenceProfileCatalog =
        serde_json::from_value(json(authored.join("catalog.json"))).unwrap();
    let bases: ItemBasePolicy =
        serde_json::from_value(json(data().join("item-bases/policy.json"))).unwrap();
    assert_eq!(catalog.profiles.len(), 1_756);
    assert_eq!(bases.templates.len(), 1_756);
    assert_eq!(policy.fields.len(), 6);
    let expected_bindings = [
        ("Armour", "29fb", "29ee", None),
        ("BlockChance", "29fc", "0002", Some("2a01")),
        ("EnergyShield", "29fd", "29ed", None),
        ("Evasion", "29fe", "29ee", None),
        ("MovementPenalty", "29ff", "0002", Some("2a02")),
        ("Ward", "2a00", "29fa", None),
    ];
    for (field, (name, stat, unit, presence)) in policy.fields.iter().zip(expected_bindings) {
        assert_eq!(field.source_field, name);
        assert_eq!(field.stat.key().as_str(), format!("def.000000000000{stat}"));
        assert_eq!(field.unit.key().as_str(), format!("def.000000000000{unit}"));
        assert_eq!(
            field
                .presence
                .as_ref()
                .map(|id| id.key().as_str().to_owned()),
            presence.map(|key| format!("def.000000000000{key}"))
        );
        match (&field.when_absent, presence) {
            (DefenceFieldAbsence::Omit, Some(_)) => {}
            (DefenceFieldAbsence::Literal(ParameterValue::Quantity(value)), None) => {
                assert_eq!(value.value(), 0.0);
                assert_eq!(value.unit(), &field.unit);
            }
            _ => panic!("optional fields omit; four raw defences use explicit zero defaults"),
        }
    }

    let output = cwd.join("defence-profile-successor");
    let report = success(publish(cwd, prior, &policy_path, &output));
    for (field, count) in [
        ("converted_profiles", 1_240),
        ("source_fields", 3_031),
        ("presence_facts", 2_480),
        ("changed_program_owners", 1_240),
    ] {
        assert_eq!(report["defence_profiles"][field], count);
    }
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(report["publication"]["schema_version"], 2);
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    assert!(!output.join("recipe.json").exists());
    let before = recipe(prior);
    let after = recipe(&output);
    check_definitions(&before, &after, &policy);
    check_rebound_inputs(prior, &output, true, &after);

    let checked = assemble_owned_recipe(after.clone(), Default::default()).unwrap();
    let compiled = CompiledRulePackage::compile(
        checked.rules().input(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let mut scratch = compiled.new_scratch();
    let profiles: BTreeMap<_, _> = catalog
        .profiles
        .iter()
        .map(|row| (row.base.as_str(), &row.profile))
        .collect();
    assert_eq!(profiles.len(), 1_756, "unique exact base identity");
    let mut restored = after.rules.clone();
    restored.definitions = before.rules.definitions.clone();
    assert_eq!(restored.owners.len(), before.rules.owners.len());
    let (mut tables, mut empty, mut absent, mut source_fields, mut defaults, mut omissions) =
        (0, 0, 0, 0, 0, 0);
    for binding in &bases.templates {
        let owner = SchemaSubject::Definition(binding.template.address());
        let old = before
            .rules
            .owners
            .iter()
            .find(|row| row.owner == owner)
            .unwrap();
        let next = restored
            .owners
            .iter_mut()
            .find(|row| row.owner == owner)
            .unwrap();
        assert!(!old.programs.is_complete());
        assert_eq!(next.programs.closure, old.programs.closure);
        assert!(
            old.programs
                .members
                .iter()
                .all(|p| p.id.as_str() != "raw-base-defence-profile")
        );
        let programs: Vec<_> = next
            .programs
            .members
            .iter()
            .filter(|p| p.id.as_str() == "raw-base-defence-profile")
            .collect();
        let source = profiles[binding.source_base.as_str()];
        let DefenceProfilePresence::Table { fields } = source else {
            assert!(
                programs.is_empty(),
                "absent {} emits no defaults or capabilities",
                binding.source_base
            );
            assert_eq!(next, old);
            absent += 1;
            continue;
        };
        assert_eq!(programs.len(), 1);
        let program = programs[0];
        assert_eq!(program.context, RuleEntityKind::EquipmentUse);
        assert!(program.reads.is_empty());
        assert!(program.effects.iter().all(|effect| effect.when.is_none()));
        let result = compiled
            .evaluate(&owner, &program.id, &[], checked.schema(), &mut scratch)
            .unwrap();
        let (mut actual_values, mut actual_presence) = (BTreeMap::new(), BTreeMap::new());
        for effect in result.effects {
            let EffectDisposition::Applied { value } = effect.disposition else {
                panic!("finite literal must apply")
            };
            match effect.effect {
                RuleEffectKind::Derive {
                    entity: RuleEntity::Current,
                    stat,
                    ..
                } => {
                    assert!(actual_values.insert(stat, value).is_none());
                }
                RuleEffectKind::Capability {
                    entity: RuleEntity::Current,
                    capability,
                    ..
                } => {
                    assert!(actual_presence.insert(capability, value).is_none());
                }
                _ => panic!("raw profile must not contribute, activate or select equipment"),
            }
        }
        let (mut expected_values, mut expected_presence) = (BTreeMap::new(), BTreeMap::new());
        for field in &policy.fields {
            let value = match fields.get(&field.source_field) {
                Some(value) => Some(ParameterValue::Quantity(
                    FiniteQuantity::new(*value, field.unit.clone()).unwrap(),
                )),
                None => match &field.when_absent {
                    DefenceFieldAbsence::Literal(value) => {
                        defaults += 1;
                        Some(value.clone())
                    }
                    DefenceFieldAbsence::Omit => {
                        omissions += 1;
                        None
                    }
                    DefenceFieldAbsence::Required => panic!("no required absence in this policy"),
                },
            };
            if let Some(value) = value {
                expected_values.insert(field.stat.clone(), value);
            }
            if let Some(capability) = &field.presence {
                expected_presence.insert(
                    capability.clone(),
                    ParameterValue::Boolean(fields.contains_key(&field.source_field)),
                );
            }
        }
        assert_eq!(
            actual_values, expected_values,
            "{} exact units/values/defaults",
            binding.source_base
        );
        assert_eq!(
            actual_presence, expected_presence,
            "{} optional field presence",
            binding.source_base
        );
        source_fields += fields.len();
        empty += usize::from(fields.is_empty());
        tables += 1;
        next.programs
            .members
            .retain(|p| p.id.as_str() != "raw-base-defence-profile");
        assert_eq!(
            serde_json::to_vec(next).unwrap(),
            serde_json::to_vec(old).unwrap(),
            "{} old program bytes/coverage changed",
            binding.source_base
        );
    }
    assert_eq!(
        (tables, empty, absent, source_fields),
        (1_240, 6, 516, 3_031)
    );
    assert_eq!(restored, before.rules, "unrelated rules/receivers changed");
    assert_eq!(report["defence_profiles"]["literal_defaults"], defaults);
    assert_eq!(report["defence_profiles"]["omitted_fields"], omissions);
    assert!(
        matches!(profiles["Fists of Stone"], DefenceProfilePresence::Table { fields } if fields.is_empty())
    );
    check_original_preservation(
        cwd,
        prior,
        &output,
        "defence-profile",
        "passive-defence",
        |_, _| {},
    );

    let published = bundle(&output);
    let rerun = cwd.join("defence-profile-rerun");
    let replay = success(publish(cwd, &output, &policy_path, &rerun));
    assert_eq!(replay["defence_profiles"]["changed_program_owners"], 0);
    for (name, bytes) in &published {
        if !["transition.json", "catalog-append.json"].contains(&name.as_str()) {
            assert_eq!(
                *bytes,
                fs::read(rerun.join(name)).unwrap(),
                "idempotent {name}"
            );
        }
    }
    assert!(!publish(cwd, prior, &policy_path, &output).status.success());
    let occupied = cwd.join("defence-profile-occupied");
    fs::create_dir(&occupied).unwrap();
    fs::write(occupied.join("keep"), b"caller").unwrap();
    assert!(
        !publish(cwd, prior, &policy_path, &occupied)
            .status
            .success()
    );
    assert_eq!(fs::read(occupied.join("keep")).unwrap(), b"caller");
    assert_eq!(fs::read_dir(&occupied).unwrap().count(), 1);
    let mut invalid = policy.clone();
    invalid.catalog_sha256 = "0".repeat(64);
    let invalid_policy = cwd.join("defence-profile-invalid-policy.json");
    fs::write(&invalid_policy, serde_json::to_vec(&invalid).unwrap()).unwrap();
    let rejected = cwd.join("defence-profile-rejected");
    assert!(
        !publish(cwd, prior, &invalid_policy, &rejected)
            .status
            .success()
    );
    assert!(
        !rejected.exists(),
        "invalid source binding must not publish a package"
    );
    assert_eq!(bundle(prior), prior_bytes);
    assert_eq!(bundle(&output), published);
    assert_eq!(bundle(&authored), authored_bytes);
    output
}
