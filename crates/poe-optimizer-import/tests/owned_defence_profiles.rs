//! Raw defensive profiles preserve whole absence, empty tables and finite field values.
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition};
use poe_optimizer_import::{
    owned_defence_profiles::*, owned_item_bases::*, owned_mapping::*, owned_recipe::*,
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
    profiles: DefenceProfileCatalog,
    policy: DefenceProfilePolicy,
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
                path: "src/Data/Bases/defence-tests.lua".into(),
                sha256: "b".repeat(64),
            }],
        };
        mapping.source.files.extend(source.files.clone());
        let mut bases = Vec::new();
        let mut profiles = Vec::new();
        let mut templates = Vec::new();
        for (name, weapon_field, profile) in [
            (
                "Armoured",
                ItemBaseWeaponField::Absent,
                DefenceProfilePresence::Table {
                    fields: BTreeMap::from([
                        ("Armour".into(), 120.5),
                        ("MovementPenalty".into(), 5.0),
                        ("BlockChance".into(), 0.0),
                    ]),
                },
            ),
            (
                "Empty",
                ItemBaseWeaponField::Absent,
                DefenceProfilePresence::Table {
                    fields: BTreeMap::new(),
                },
            ),
            (
                "Absent",
                ItemBaseWeaponField::Table,
                DefenceProfilePresence::Absent,
            ),
            (
                "Zero",
                ItemBaseWeaponField::Absent,
                DefenceProfilePresence::Table {
                    fields: BTreeMap::from([("Armour".into(), 0.0)]),
                },
            ),
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
                        code: key("unconverted-defence-rules"),
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
                item_type: "Injected equipment".into(),
                source_module: source.files[0].path.clone(),
                weapon_field,
            });
            profiles.push(DefenceProfileRow {
                base: name.into(),
                profile,
            });
            templates.push(id);
        }
        let mut units = Vec::new();
        for dimension in [UnitDimension::Rating, UnitDimension::PercentagePoints] {
            let id: UnitDefId = registry.allocate_definition().unwrap();
            recipe
                .schema
                .definitions
                .push(DefinitionDescriptor::Unit(DefinitionEntry {
                    id: id.clone(),
                    schema: SchemaState::Known(UnitSchema { dimension }),
                }));
            units.push(id);
        }
        let mut fields = Vec::new();
        for (name, unit) in [
            ("Armour", units[0].clone()),
            ("BlockChance", units[1].clone()),
            ("MovementPenalty", units[1].clone()),
        ] {
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
            let presence = if name == "Armour" {
                None
            } else {
                let id: CapabilityDefId = registry.allocate_definition().unwrap();
                recipe
                    .schema
                    .definitions
                    .push(DefinitionDescriptor::Capability(DefinitionEntry {
                        id: id.clone(),
                        schema: SchemaState::Known(CapabilitySchema {
                            targets: vec![RuleEntityKind::EquipmentUse],
                        }),
                    }));
                Some(id)
            };
            fields.push(DefenceProfileField {
                source_field: name.into(),
                stat,
                unit: unit.clone(),
                presence,
                when_absent: if name == "Armour" {
                    DefenceFieldAbsence::Literal(ParameterValue::Quantity(
                        FiniteQuantity::new(0.0, unit).unwrap(),
                    ))
                } else {
                    DefenceFieldAbsence::Omit
                },
            });
        }
        recipe.registry = registry.input().clone();
        let mut f = Self {
            recipe,
            mapping,
            templates,
            bases: ItemBaseCatalog {
                schema_version: 1,
                source: source.clone(),
                bases,
            },
            profiles: DefenceProfileCatalog {
                schema_version: 1,
                source,
                base_catalog_sha256: String::new(),
                profiles,
            },
            policy: DefenceProfilePolicy {
                schema_version: 1,
                version: key("defence-test-fields"),
                catalog_sha256: String::new(),
                base_catalog_sha256: String::new(),
                fields,
            },
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
        limits: DefenceProfileLimits,
    ) -> Result<StagedDefenceProfileRecipe, DefenceProfileError> {
        let (base, mapping) = self.checked();
        compile_owned_defence_profiles(
            &base,
            &mapping,
            &wire(&self.bases),
            &wire(&self.profiles),
            &self.policy,
            limits,
        )
    }
    fn compile(&self) -> Result<StagedDefenceProfileRecipe, DefenceProfileError> {
        self.compile_with(Default::default())
    }
    fn fields(&mut self, index: usize) -> &mut BTreeMap<String, f64> {
        let DefenceProfilePresence::Table { fields } = &mut self.profiles.profiles[index].profile
        else {
            panic!("table")
        };
        fields
    }
}
fn bad(f: &Fixture, expected: &str) {
    let error = f.compile().unwrap_err();
    assert!(error.to_string().contains(expected), "{error}");
}
fn observed(
    f: &Fixture,
    recipe: &OwnedRecipeInput,
    index: usize,
) -> BTreeMap<String, ParameterValue> {
    let base = assemble_owned_recipe(recipe.clone(), Default::default()).unwrap();
    let compiled =
        CompiledRulePackage::compile(base.rules().input(), base.schema(), Default::default())
            .unwrap();
    let result = compiled
        .evaluate(
            &subject(&f.templates[index]),
            &key("raw-base-defence-profile"),
            &[],
            base.schema(),
            &mut compiled.new_scratch(),
        )
        .unwrap();
    result
        .effects
        .into_iter()
        .map(|effect| {
            let EffectDisposition::Applied { value } = effect.disposition else {
                panic!("raw literal")
            };
            let name = match effect.effect {
                RuleEffectKind::Derive {
                    entity: RuleEntity::Current,
                    stat,
                    ..
                } => f
                    .policy
                    .fields
                    .iter()
                    .find(|field| field.stat == stat)
                    .unwrap()
                    .source_field
                    .clone(),
                RuleEffectKind::Capability {
                    entity: RuleEntity::Current,
                    capability,
                    ..
                } => format!(
                    "{}-present",
                    f.policy
                        .fields
                        .iter()
                        .find(|field| field.presence.as_ref() == Some(&capability))
                        .unwrap()
                        .source_field
                ),
                _ => panic!("only current equipment facts"),
            };
            (name, value)
        })
        .collect()
}
fn quantity(f: &Fixture, name: &str, value: f64) -> ParameterValue {
    ParameterValue::Quantity(
        FiniteQuantity::new(
            value,
            f.policy
                .fields
                .iter()
                .find(|field| field.source_field == name)
                .unwrap()
                .unit
                .clone(),
        )
        .unwrap(),
    )
}
#[test]
fn absent_empty_authored_zero_and_nonzero_profiles_remain_distinct() {
    let f = Fixture::new();
    let (base, _) = f.checked();
    let out = f.compile().unwrap();
    assert_eq!(
        (
            out.receipt.converted_profiles,
            out.receipt.source_fields,
            out.receipt.literal_defaults,
            out.receipt.omitted_fields,
            out.receipt.presence_facts,
            out.receipt.changed_program_owners
        ),
        (3, 4, 1, 4, 6, 3)
    );
    assert_eq!(out.successor.registry, *base.registry().input());
    assert_eq!(out.successor.schema, *base.schema().input());
    assert_eq!(out.successor.routing, *base.routing().input());
    assert_eq!(
        observed(&f, &out.successor, 0),
        BTreeMap::from([
            ("Armour".into(), quantity(&f, "Armour", 120.5)),
            ("BlockChance".into(), quantity(&f, "BlockChance", 0.0)),
            ("BlockChance-present".into(), ParameterValue::Boolean(true)),
            (
                "MovementPenalty".into(),
                quantity(&f, "MovementPenalty", 5.0)
            ),
            (
                "MovementPenalty-present".into(),
                ParameterValue::Boolean(true)
            ),
        ])
    );
    let empty = BTreeMap::from([
        ("Armour".into(), quantity(&f, "Armour", 0.0)),
        ("BlockChance-present".into(), ParameterValue::Boolean(false)),
        (
            "MovementPenalty-present".into(),
            ParameterValue::Boolean(false),
        ),
    ]);
    assert_eq!(observed(&f, &out.successor, 1), empty);
    assert_eq!(observed(&f, &out.successor, 3), empty);
    for template in &f.templates {
        let owner = subject(template);
        let old = f
            .recipe
            .rules
            .owners
            .iter()
            .find(|row| row.owner == owner)
            .unwrap();
        let new = out
            .successor
            .rules
            .owners
            .iter()
            .find(|row| row.owner == owner)
            .unwrap();
        assert_eq!(new.programs.closure, old.programs.closure);
        if template == &f.templates[2] {
            assert_eq!(new, old);
        } else {
            assert_eq!(new.programs.members.len(), old.programs.members.len() + 1);
        }
    }
}
#[test]
fn replay_is_idempotent_and_data_changes_require_an_explicit_new_recipe() {
    let mut f = Fixture::new();
    let first = f.compile().unwrap();
    f.recipe = first.successor.clone();
    let second = f.compile().unwrap();
    assert_eq!(second.successor, first.successor);
    assert_eq!(second.receipt.changed_program_owners, 0);
    f.fields(0).insert("Armour".into(), 121.25);
    f.repin();
    bad(&f, "preservation");
    let mut changed = Fixture::new();
    changed.fields(0).insert("Armour".into(), 121.25);
    changed.repin();
    let result = changed.compile().unwrap();
    assert_eq!(
        observed(&changed, &result.successor, 0)["Armour"],
        quantity(&changed, "Armour", 121.25)
    );
}
#[test]
fn every_base_requires_exactly_one_explicit_presence_row() {
    for action in 0..4 {
        let mut f = Fixture::new();
        match action {
            0 => {
                f.profiles.profiles.remove(2);
            }
            1 => {
                f.profiles.profiles.push(f.profiles.profiles[2].clone());
            }
            2 => {
                f.profiles.profiles[2].base = "unknown base".into();
            }
            _ => {
                f.bases.bases[2].name = "unmapped base".into();
            }
        }
        f.repin();
        assert!(f.compile().is_err());
    }
    let mut f = Fixture::new();
    // Defence absence is independent of the older weapon-table discriminator.
    for base in &mut f.bases.bases {
        base.weapon_field = ItemBaseWeaponField::Table;
    }
    f.repin();
    let out = f.compile().unwrap();
    assert_eq!(out.receipt.converted_profiles, 3);
}
#[test]
fn reviewed_field_membership_required_values_and_exact_types_reject() {
    let mut f = Fixture::new();
    f.fields(0).insert("Unknown".into(), 3.0);
    f.repin();
    bad(&f, "unreviewed source field");
    let mut f = Fixture::new();
    f.policy.fields[0].when_absent = DefenceFieldAbsence::Required;
    bad(&f, "required profile field absent");
    let mut f = Fixture::new();
    f.policy.fields[0].when_absent = DefenceFieldAbsence::Literal(ParameterValue::Boolean(false));
    bad(&f, "exact output unit");
    let mut f = Fixture::new();
    f.policy.fields[0].unit = f.policy.fields[1].unit.clone();
    bad(&f, "type, unit or scope");
    let mut f = Fixture::new();
    f.policy.fields.push(f.policy.fields[0].clone());
    bad(&f, "duplicate or invalid field/stat");
    let mut f = Fixture::new();
    f.policy.fields[2].presence = f.policy.fields[1].presence.clone();
    bad(&f, "presence capability scope or duplicate");
    let mut f = Fixture::new();
    f.policy.fields[0].source_field = "Unobserved".into();
    bad(&f, "unreviewed source field");
    let mut f = Fixture::new();
    let id = f.policy.fields[0].stat.clone();
    for descriptor in &mut f.recipe.schema.definitions {
        if let DefinitionDescriptor::Stat(entry) = descriptor
            && entry.id == id
            && let SchemaState::Known(schema) = &mut entry.schema
        {
            schema.targets = vec![RuleEntityKind::Actor];
        }
    }
    f.rebind();
    bad(&f, "type, unit or scope");
}
#[test]
fn hashes_source_bindings_limits_and_duplicate_nonfinite_fields_reject() {
    let mut f = Fixture::new();
    f.policy.catalog_sha256 = "0".repeat(64);
    assert!(matches!(f.compile(), Err(DefenceProfileError::Binding)));
    let mut f = Fixture::new();
    f.profiles.source.revision = "wrong".into();
    f.repin();
    assert!(matches!(f.compile(), Err(DefenceProfileError::Binding)));
    let f = Fixture::new();
    for limits in [
        DefenceProfileLimits {
            max_profiles: 3,
            ..Default::default()
        },
        DefenceProfileLimits {
            max_fields: 2,
            ..Default::default()
        },
        DefenceProfileLimits {
            max_work: 1,
            ..Default::default()
        },
        DefenceProfileLimits {
            max_catalog_bytes: 1,
            ..Default::default()
        },
    ] {
        assert!(f.compile_with(limits).is_err());
    }
    for text in [
        r#"{"base":"x","profile":{"kind":"table","value":{"fields":{"Armour":1,"Armour":2}}}}"#,
        r#"{"base":"x","profile":{"kind":"table","value":{"fields":{"Armour":1e999}}}}"#,
        r#"{"base":"x","profile":{"kind":"absent","value":{"fields":{"Armour":0}}}}"#,
        r#"{"base":"x","profile":{"kind":"table","value":{"fields":{},"extra":0}}}"#,
    ] {
        assert!(
            serde_json::from_str::<DefenceProfileRow>(text).is_err(),
            "{text}"
        );
    }
}
