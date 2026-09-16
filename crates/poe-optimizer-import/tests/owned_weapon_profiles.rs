//! Raw profiles are injected data, never complete local/final weapon evaluation.
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition};
use poe_optimizer_import::{
    owned_item_bases::*, owned_mapping::*, owned_recipe::*, owned_weapon_profiles::*,
};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf};

fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn load<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(
        &fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../data/owned/poe2/3887ae68/current")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
fn wire<T: serde::Serialize>(value: &T) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(value).unwrap();
    bytes.push(b'\n');
    bytes
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn partial<T>(owner: &SchemaSubject) -> DeclaredSet<T> {
    DeclaredSet::partial(
        vec![],
        vec![SchemaGap {
            subject: owner.clone(),
            facet: SchemaFacet::InputSchema,
            code: key("unconverted-inputs"),
        }],
    )
}
fn item(id: ItemTemplateDefId) -> DefinitionDescriptor {
    let owner = SchemaSubject::Definition(id.address());
    DefinitionDescriptor::ItemTemplate(DefinitionEntry {
        id,
        schema: SchemaState::Known(ItemTemplateSchema {
            item_level: IntegerRange {
                minimum: BoundedInteger::new(0).unwrap(),
                maximum: BoundedInteger::new(100).unwrap(),
            },
            equipment_slots: partial(&owner),
            socket_destinations: partial(&owner),
            modifiers: partial(&owner),
            quality: QualityUseSchema {
                presence: QualityPresence::Optional,
                allowed_kinds: partial(&owner),
            },
            declarations: DeclaredSlots {
                parameters: partial(&owner),
                choices: partial(&owner),
                grants: partial(&owner),
                actors: partial(&owner),
                skill_grants: partial(&owner),
                outputs: partial(&owner),
                sockets: partial(&owner),
            },
        }),
    })
}
fn subject(id: &ItemTemplateDefId) -> SchemaSubject {
    SchemaSubject::Definition(id.address())
}
struct Fixture {
    recipe: OwnedRecipeInput,
    mapping: MappingPackageInput,
    bases: ItemBaseCatalog,
    profiles: WeaponProfileCatalog,
    policy: WeaponProfilePolicy,
    templates: Vec<ItemTemplateDefId>,
}
impl Fixture {
    fn new() -> Self {
        let mut recipe: OwnedRecipeInput = load("recipe.json");
        let mut mapping: MappingPackageInput = load("mapping.json");
        let mut registry =
            OwnedIdRegistry::new(recipe.registry.clone(), Default::default()).unwrap();
        let source = SourcePin {
            system: mapping.source.system,
            revision: mapping.source.revision.clone(),
            files: vec![SourceFilePin {
                path: "src/Data/Bases/profile-tests.lua".into(),
                sha256: "a".repeat(64),
            }],
        };
        mapping.source.files.extend(source.files.clone());
        let mut bases = vec![];
        let mut templates = vec![];
        for (name, shape) in [
            ("Alpha Profile", ItemBaseWeaponField::Table),
            ("Beta Profile", ItemBaseWeaponField::Table),
            ("No Profile", ItemBaseWeaponField::Absent),
        ] {
            let id: ItemTemplateDefId = registry.allocate_definition().unwrap();
            recipe.schema.definitions.push(item(id.clone()));
            let owner = subject(&id);
            recipe.rules.owners.push(DefinitionRules {
                owner: owner.clone(),
                programs: DeclaredSet::partial(
                    vec![],
                    vec![SchemaGap {
                        subject: owner.clone(),
                        facet: SchemaFacet::GameRules,
                        code: key("unconverted-item-rules"),
                    }],
                ),
            });
            mapping.entries.push(MappingEntry {
                source: ExternalSelector::Definition(ExternalOwnerSelector::ItemTemplate {
                    base: SourceComponent::Text(name.into()),
                    prototype: SourceComponent::Missing,
                    variant: SourceComponent::Missing,
                }),
                outcome: MappingOutcome::Mapped {
                    target: owner,
                    basis: MappingBasis::Exact,
                },
            });
            bases.push(ItemBaseRow {
                name: name.into(),
                item_type: "Injected Profile Type".into(),
                source_module: source.files[0].path.clone(),
                weapon_field: shape,
            });
            templates.push(id);
        }
        let mut units = BTreeMap::new();
        for (name, dimension) in [
            ("rate", UnitDimension::Rate),
            ("crit", UnitDimension::PercentagePoints),
            ("damage", UnitDimension::Damage),
            ("distance", UnitDimension::Distance),
            ("time", UnitDimension::Time),
        ] {
            let id: UnitDefId = registry.allocate_definition().unwrap();
            recipe
                .schema
                .definitions
                .push(DefinitionDescriptor::Unit(DefinitionEntry {
                    id: id.clone(),
                    schema: SchemaState::Known(UnitSchema { dimension }),
                }));
            units.insert(name, id);
        }
        let reload_presence: CapabilityDefId = registry.allocate_definition().unwrap();
        recipe
            .schema
            .definitions
            .push(DefinitionDescriptor::Capability(DefinitionEntry {
                id: reload_presence.clone(),
                schema: SchemaState::Known(CapabilitySchema {
                    targets: vec![RuleEntityKind::EquipmentUse],
                }),
            }));
        let mut fields = vec![];
        for (name, unit_name) in [
            ("AttackRateBase", "rate"),
            ("CritChanceBase", "crit"),
            ("Range", "distance"),
            ("ReloadTimeBase", "time"),
            ("PhysicalMin", "damage"),
            ("PhysicalMax", "damage"),
            ("FireMin", "damage"),
            ("FireMax", "damage"),
            ("ColdMin", "damage"),
            ("ColdMax", "damage"),
            ("LightningMin", "damage"),
            ("LightningMax", "damage"),
            ("ChaosMin", "damage"),
            ("ChaosMax", "damage"),
        ] {
            let unit = units[unit_name].clone();
            let stat: StatDefId = registry.allocate_definition().unwrap();
            recipe
                .schema
                .definitions
                .push(DefinitionDescriptor::Stat(DefinitionEntry {
                    id: stat.clone(),
                    schema: SchemaState::Known(StatSchema {
                        value: ComputedValueType::Quantity { unit: unit.clone() },
                        targets: vec![RuleEntityKind::EquipmentUse],
                    }),
                }));
            let when_absent = if name == "ReloadTimeBase" {
                WeaponFieldAbsence::Omit
            } else if unit_name == "damage" {
                WeaponFieldAbsence::Literal(ParameterValue::Quantity(
                    FiniteQuantity::new(0.0, unit.clone()).unwrap(),
                ))
            } else {
                WeaponFieldAbsence::Required
            };
            fields.push(WeaponProfileField {
                source_field: name.into(),
                stat,
                unit,
                when_absent,
                presence: (name == "ReloadTimeBase").then(|| reload_presence.clone()),
            });
        }
        recipe.registry = registry.input().clone();
        let profiles = vec![
            WeaponProfileRow {
                base: "Alpha Profile".into(),
                fields: BTreeMap::from([
                    ("AttackRateBase".into(), 1.4),
                    ("CritChanceBase".into(), 5.0),
                    ("Range".into(), 15.0),
                    ("ReloadTimeBase".into(), 0.85),
                    ("PhysicalMin".into(), 56.0),
                    ("PhysicalMax".into(), 84.0),
                    ("FireMin".into(), 1.0),
                    ("FireMax".into(), 3.0),
                    ("ColdMin".into(), 2.0),
                    ("ColdMax".into(), 4.0),
                    ("LightningMin".into(), 0.0),
                    ("LightningMax".into(), 420.0),
                    ("ChaosMin".into(), 7.0),
                    ("ChaosMax".into(), 19.0),
                ]),
            },
            WeaponProfileRow {
                base: "Beta Profile".into(),
                fields: BTreeMap::from([
                    ("AttackRateBase".into(), 1.65),
                    ("CritChanceBase".into(), 0.0),
                    ("Range".into(), 120.0),
                    ("FireMin".into(), 5.0),
                    ("FireMax".into(), 95.0),
                ]),
            },
        ];
        let mut f = Self {
            recipe,
            mapping,
            bases: ItemBaseCatalog {
                schema_version: 1,
                source: source.clone(),
                bases,
            },
            profiles: WeaponProfileCatalog {
                schema_version: 1,
                source,
                base_catalog_sha256: String::new(),
                profiles,
            },
            policy: WeaponProfilePolicy {
                schema_version: 1,
                version: key("reviewed-raw-fields"),
                catalog_sha256: String::new(),
                base_catalog_sha256: String::new(),
                fields,
            },
            templates,
        };
        f.rebind();
        f.repin();
        f
    }
    fn rebind(&mut self) {
        let schema = poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage::new(
            self.recipe.schema.clone(),
            Default::default(),
        )
        .unwrap();
        self.recipe.rules.definitions = schema.identity().clone();
        self.recipe.routing.definitions = schema.identity().clone();
    }
    fn repin(&mut self) {
        self.policy.base_catalog_sha256 = hash(&wire(&self.bases));
        self.profiles.base_catalog_sha256 = self.policy.base_catalog_sha256.clone();
        self.policy.catalog_sha256 = hash(&wire(&self.profiles));
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
    fn compile_with(
        &self,
        limits: WeaponProfileLimits,
    ) -> Result<StagedWeaponProfileRecipe, WeaponProfileError> {
        let (base, mapping) = self.checked();
        compile_owned_weapon_profiles(
            &base,
            &mapping,
            &wire(&self.bases),
            &wire(&self.profiles),
            &self.policy,
            limits,
        )
    }
    fn compile(&self) -> Result<StagedWeaponProfileRecipe, WeaponProfileError> {
        self.compile_with(Default::default())
    }
}
fn bad(f: &Fixture, text: &str) {
    let error = f.compile().unwrap_err();
    assert!(error.to_string().contains(text), "{error}");
}
fn evaluated(
    f: &Fixture,
    out: &OwnedRecipeInput,
    index: usize,
) -> BTreeMap<StatDefId, ParameterValue> {
    let checked = assemble_owned_recipe(out.clone(), Default::default()).unwrap();
    let compiled = CompiledRulePackage::compile(
        checked.rules().input(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let mut scratch = compiled.new_scratch();
    let evaluated = compiled
        .evaluate(
            &subject(&f.templates[index]),
            &key("raw-base-attack-profile"),
            &[],
            checked.schema(),
            &mut scratch,
        )
        .unwrap();
    evaluated
        .effects
        .iter()
        .filter_map(|e| match (&e.effect, &e.disposition) {
            (
                RuleEffectKind::Derive {
                    entity: RuleEntity::Current,
                    stat,
                    ..
                },
                EffectDisposition::Applied { value },
            ) => Some((stat.clone(), value.clone())),
            (
                RuleEffectKind::Capability {
                    entity: RuleEntity::Current,
                    ..
                },
                EffectDisposition::Applied {
                    value: ParameterValue::Boolean(_),
                },
            ) => None,
            other => panic!("unexpected raw effect {other:?}"),
        })
        .collect()
}

#[test]
fn all_fourteen_channels_preserve_raw_values_zero_absence_and_every_unrelated_artifact() {
    let f = Fixture::new();
    let (base, _) = f.checked();
    let out = f.compile().unwrap();
    assert_eq!(out.receipt.converted_profiles, 2);
    assert_eq!(out.receipt.source_fields, 19);
    assert_eq!(out.receipt.literal_defaults, 8);
    assert_eq!(out.receipt.omitted_fields, 1);
    assert_eq!(out.receipt.presence_facts, 2);
    assert_eq!(out.receipt.changed_program_owners, 2);
    assert_eq!(out.successor.schema, *base.schema().input());
    assert_eq!(out.successor.registry, *base.registry().input());
    assert_eq!(out.successor.routing, *base.routing().input());
    assert_eq!(out.successor.rules.tables, base.rules().input().tables);
    assert_eq!(
        out.successor.rules.receivers,
        base.rules().input().receivers
    );
    assert_eq!(
        out.successor.rules.operations_version,
        base.rules().input().operations_version
    );
    for owner in &base.rules().input().owners {
        let after = out
            .successor
            .rules
            .owners
            .iter()
            .find(|r| r.owner == owner.owner)
            .unwrap();
        assert_eq!(after.programs.closure, owner.programs.closure);
        for program in &owner.programs.members {
            assert!(after.programs.members.contains(program));
        }
    }
    for n in 0..2 {
        let values = evaluated(&f, &out.successor, n);
        for field in &f.policy.fields {
            if let Some(source) = f.profiles.profiles[n].fields.get(&field.source_field) {
                assert_eq!(
                    values[&field.stat],
                    ParameterValue::Quantity(
                        FiniteQuantity::new(*source, field.unit.clone()).unwrap()
                    )
                );
            } else {
                match &field.when_absent {
                    WeaponFieldAbsence::Literal(expected) => {
                        assert_eq!(&values[&field.stat], expected)
                    }
                    WeaponFieldAbsence::Omit => assert!(!values.contains_key(&field.stat)),
                    WeaponFieldAbsence::Required => panic!("required fixture field"),
                }
            }
        }
    }
    let no_profile = out
        .successor
        .rules
        .owners
        .iter()
        .find(|o| o.owner == subject(&f.templates[2]))
        .unwrap();
    assert!(no_profile.programs.members.is_empty());
    assert!(!no_profile.programs.is_complete());
}

#[test]
fn optional_presence_is_true_for_present_zero_and_false_for_omitted_reload() {
    let mut f = Fixture::new();
    f.profiles.profiles[0]
        .fields
        .insert("ReloadTimeBase".into(), 0.0);
    f.repin();
    let out = f.compile().unwrap();
    for n in 0..2 {
        let row = out
            .successor
            .rules
            .owners
            .iter()
            .find(|o| o.owner == subject(&f.templates[n]))
            .unwrap();
        let p = row.programs.members.last().unwrap();
        let effect = p
            .effects
            .iter()
            .find_map(|e| {
                if let RuleEffectKind::Capability { enabled, .. } = &e.effect {
                    Some(enabled)
                } else {
                    None
                }
            })
            .unwrap();
        let node = p.nodes.iter().find(|v| &v.id == effect).unwrap();
        assert_eq!(
            node.expression,
            RuleExpression::Literal {
                value: ParameterValue::Boolean(n == 0)
            }
        );
    }
    assert_eq!(
        evaluated(&f, &out.successor, 0)[&f.policy.fields[3].stat],
        ParameterValue::Quantity(
            FiniteQuantity::new(0.0, f.policy.fields[3].unit.clone()).unwrap()
        )
    );
}

#[test]
fn idempotence_and_source_policy_order_are_stable_while_injected_values_change_outputs() {
    let mut f = Fixture::new();
    let first = f.compile().unwrap();
    f.profiles.profiles.reverse();
    f.policy.fields.reverse();
    f.repin();
    let reordered = f.compile().unwrap();
    assert_eq!(first.successor, reordered.successor);
    f.recipe = first.successor.clone();
    let repeated = f.compile().unwrap();
    assert_eq!(first.successor, repeated.successor);
    assert_eq!(repeated.receipt.changed_program_owners, 0);
    f.profiles.profiles[0]
        .fields
        .insert("AttackRateBase".into(), 2.75);
    f.repin();
    bad(&f, "preservation");
    let mut f = Fixture::new();
    f.profiles.profiles[0]
        .fields
        .insert("AttackRateBase".into(), 2.75);
    f.repin();
    let changed = f.compile().unwrap();
    assert_eq!(
        evaluated(&f, &changed.successor, 0)[&f.policy.fields[0].stat],
        ParameterValue::Quantity(
            FiniteQuantity::new(2.75, f.policy.fields[0].unit.clone()).unwrap()
        )
    );
    f.policy.fields[4].when_absent = WeaponFieldAbsence::Literal(ParameterValue::Quantity(
        FiniteQuantity::new(11.0, f.policy.fields[4].unit.clone()).unwrap(),
    ));
    let changed = f.compile().unwrap();
    assert_eq!(
        evaluated(&f, &changed.successor, 1)[&f.policy.fields[4].stat],
        ParameterValue::Quantity(
            FiniteQuantity::new(11.0, f.policy.fields[4].unit.clone()).unwrap()
        )
    );
}

#[test]
fn whole_profile_membership_and_all_field_membership_must_match_the_pinned_base_catalog() {
    let mut f = Fixture::new();
    f.profiles.profiles.pop();
    f.repin();
    bad(&f, "profile membership");
    let mut f = Fixture::new();
    f.profiles.profiles[1].base = "No Profile".into();
    f.repin();
    bad(&f, "profile membership");
    let mut f = Fixture::new();
    f.profiles.profiles.push(f.profiles.profiles[0].clone());
    f.repin();
    bad(&f, "duplicate");
    let mut f = Fixture::new();
    f.bases.bases[1].weapon_field = ItemBaseWeaponField::Unsupported;
    f.repin();
    bad(&f, "profile membership");
    let mut f = Fixture::new();
    f.profiles.profiles[1].fields.remove("AttackRateBase");
    f.repin();
    bad(&f, "required profile field absent");
    let mut f = Fixture::new();
    f.profiles.profiles[1]
        .fields
        .insert("UnreviewedCoefficient".into(), 3.0);
    f.repin();
    bad(&f, "unreviewed");
    let mut f = Fixture::new();
    f.policy.fields[0].source_field = "OtherCoefficient".into();
    bad(&f, "unreviewed");
    let mut f = Fixture::new();
    for row in &mut f.profiles.profiles {
        row.fields.remove("ReloadTimeBase");
    }
    f.repin();
    bad(&f, "field membership");
}

#[test]
fn source_bytes_prior_catalog_provenance_and_duplicate_fields_are_bound() {
    let mut f = Fixture::new();
    f.policy.catalog_sha256 = "0".repeat(64);
    bad(&f, "binding differs");
    let mut f = Fixture::new();
    f.policy.base_catalog_sha256 = "0".repeat(64);
    bad(&f, "binding differs");
    let mut f = Fixture::new();
    f.profiles.base_catalog_sha256 = "0".repeat(64);
    f.policy.catalog_sha256 = hash(&wire(&f.profiles));
    bad(&f, "binding differs");
    let mut f = Fixture::new();
    f.profiles.source.files[0].sha256 = "0".repeat(64);
    f.repin();
    bad(&f, "binding differs");
    let mut f = Fixture::new();
    f.bases.source.files[0].sha256 = "0".repeat(64);
    f.profiles.source = f.bases.source.clone();
    f.repin();
    bad(&f, "binding differs");
    let f = Fixture::new();
    let (base, mapping) = f.checked();
    let bytes = String::from_utf8(wire(&f.profiles)).unwrap().replacen(
        "\"AttackRateBase\":1.4",
        "\"AttackRateBase\":1.4,\"AttackRateBase\":1.4",
        1,
    );
    let mut policy = f.policy.clone();
    policy.catalog_sha256 = hash(bytes.as_bytes());
    let error = compile_owned_weapon_profiles(
        &base,
        &mapping,
        &wire(&f.bases),
        bytes.as_bytes(),
        &policy,
        Default::default(),
    )
    .unwrap_err();
    assert!(error.to_string().contains("duplicate"));
}

#[test]
fn units_target_scope_and_field_aliases_are_rejected() {
    let mut f = Fixture::new();
    f.policy.fields[0].unit = f.policy.fields[4].unit.clone();
    bad(&f, "type, unit or scope");
    let mut f = Fixture::new();
    f.policy.fields[5].stat = f.policy.fields[4].stat.clone();
    bad(&f, "duplicate");
    let mut f = Fixture::new();
    f.policy.fields[5].source_field = f.policy.fields[4].source_field.clone();
    bad(&f, "duplicate");
    let mut f = Fixture::new();
    f.policy.fields[4].when_absent =
        WeaponFieldAbsence::Literal(ParameterValue::Integer(BoundedInteger::new(0).unwrap()));
    bad(&f, "exact output unit");
    let mut f = Fixture::new();
    f.policy.fields[4].when_absent = WeaponFieldAbsence::Literal(ParameterValue::Quantity(
        FiniteQuantity::new(0.0, f.policy.fields[0].unit.clone()).unwrap(),
    ));
    bad(&f, "exact output unit");
    let mut f = Fixture::new();
    let stat = f.policy.fields[0].stat.clone();
    for d in &mut f.recipe.schema.definitions {
        if let DefinitionDescriptor::Stat(e) = d
            && e.id == stat
        {
            let SchemaState::Known(s) = &mut e.schema else {
                unreachable!()
            };
            s.targets = vec![RuleEntityKind::Actor];
        }
    }
    f.rebind();
    bad(&f, "type, unit or scope");
    let mut f = Fixture::new();
    f.policy.fields[0].presence = f.policy.fields[3].presence.clone();
    bad(&f, "presence capability scope or duplicate");
}

#[test]
fn existing_complete_owner_and_competing_numeric_or_presence_writers_are_not_replaced() {
    let mut f = Fixture::new();
    let wanted = subject(&f.templates[0]);
    f.recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == wanted)
        .unwrap()
        .programs = DeclaredSet::complete(vec![]);
    bad(&f, "preservation");
    let mut f = Fixture::new();
    let out = f.compile().unwrap();
    let mut raw = out
        .successor
        .rules
        .owners
        .iter()
        .find(|o| o.owner == subject(&f.templates[0]))
        .unwrap()
        .programs
        .members
        .last()
        .unwrap()
        .clone();
    raw.id = key("competing-numerical-writer");
    f.recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == subject(&f.templates[2]))
        .unwrap()
        .programs
        .members
        .push(raw);
    bad(&f, "preservation");
    let presence_only = f
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == subject(&f.templates[2]))
        .unwrap()
        .programs
        .members
        .last_mut()
        .unwrap();
    presence_only.id = key("competing-presence-writer");
    presence_only
        .effects
        .retain(|e| matches!(e.effect, RuleEffectKind::Capability { .. }));
    assert_eq!(presence_only.effects.len(), 1);
    bad(&f, "preservation");
    let mut f = Fixture::new();
    let out = f.compile().unwrap();
    f.recipe = out.successor;
    let row = f
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == subject(&f.templates[0]))
        .unwrap();
    let program = row.programs.members.last_mut().unwrap();
    program.nodes[0].expression = RuleExpression::Literal {
        value: ParameterValue::Quantity(
            FiniteQuantity::new(99.0, f.policy.fields[0].unit.clone()).unwrap(),
        ),
    };
    bad(&f, "preservation");
}

#[test]
fn exact_template_source_identity_and_existing_rule_coverage_are_required() {
    let mut f = Fixture::new();
    let id = subject(&f.templates[0]);
    let entry = f
        .mapping
        .entries
        .iter_mut()
        .find(|e| matches!(&e.outcome,MappingOutcome::Mapped{target,..}if target==&id))
        .unwrap();
    entry.outcome = MappingOutcome::Unmapped {
        issue: key("unresolved"),
    };
    bad(&f, "exact base identity");
    let mut f = Fixture::new();
    let id = subject(&f.templates[0]);
    let entry = f
        .mapping
        .entries
        .iter_mut()
        .find(|e| matches!(&e.outcome,MappingOutcome::Mapped{target,..}if target==&id))
        .unwrap();
    let MappingOutcome::Mapped { basis, .. } = &mut entry.outcome else {
        unreachable!()
    };
    *basis = MappingBasis::ReviewedAlias {
        reason: key("not-exact-base"),
    };
    bad(&f, "exact base identity");
    let mut f = Fixture::new();
    let id = subject(&f.templates[0]);
    f.recipe.rules.owners.retain(|o| o.owner != id);
    bad(&f, "preservation");
}

#[test]
fn independent_wire_profile_field_and_work_limits_are_enforced() {
    let f = Fixture::new();
    for limits in [
        WeaponProfileLimits {
            max_catalog_bytes: 1,
            ..Default::default()
        },
        WeaponProfileLimits {
            max_base_catalog_bytes: 1,
            ..Default::default()
        },
        WeaponProfileLimits {
            max_policy_bytes: 1,
            ..Default::default()
        },
        WeaponProfileLimits {
            max_profiles: 1,
            ..Default::default()
        },
        WeaponProfileLimits {
            max_bases: 1,
            ..Default::default()
        },
        WeaponProfileLimits {
            max_fields: 1,
            ..Default::default()
        },
        WeaponProfileLimits {
            max_work: 1,
            ..Default::default()
        },
        WeaponProfileLimits {
            max_work: 0,
            ..Default::default()
        },
    ] {
        assert!(f.compile_with(limits).is_err());
    }
}
