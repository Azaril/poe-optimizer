//! Finite raw item attack-profile inputs, distinct from local/final weapon stats.
//!
//! Every source field and absence outcome is explicitly bound by caller policy.
//! Missing whole profiles never produce zeros. Raw catalog absence is retained;
//! any literal fallback is a reviewed input to later assembly, not source data.
use crate::{
    owned_item_bases::{ItemBaseCatalog, ItemBaseWeaponField, OWNED_ITEM_BASE_VERSION},
    owned_mapping::*,
    owned_recipe::*,
};
use poe_optimizer_core::{
    data::DataIdentity,
    owned_build::ParameterValue,
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{MapAccess, Visitor},
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

pub const OWNED_WEAPON_PROFILE_VERSION: u32 = 1;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponProfileCatalog {
    pub schema_version: u32,
    pub source: SourcePin,
    pub base_catalog_sha256: String,
    pub profiles: Vec<WeaponProfileRow>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponProfileRow {
    pub base: String,
    #[serde(deserialize_with = "unique_fields")]
    pub fields: BTreeMap<String, f64>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponProfilePolicy {
    pub schema_version: u32,
    pub version: OwnedDefinitionKey,
    pub catalog_sha256: String,
    pub base_catalog_sha256: String,
    pub fields: Vec<WeaponProfileField>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponProfileField {
    pub source_field: String,
    pub stat: StatDefId,
    pub unit: UnitDefId,
    pub when_absent: WeaponFieldAbsence,
    /// Optional explicit raw-field presence, scoped to a known profile. This
    /// distinguishes an inapplicable optional field from a zero quantity.
    pub presence: Option<CapabilityDefId>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum WeaponFieldAbsence {
    Required,
    Omit,
    Literal(ParameterValue),
}
#[derive(Clone, Copy, Debug)]
pub struct WeaponProfileLimits {
    pub max_catalog_bytes: usize,
    pub max_base_catalog_bytes: usize,
    pub max_policy_bytes: usize,
    pub max_profiles: usize,
    pub max_bases: usize,
    pub max_fields: usize,
    pub max_work: usize,
    pub recipe: OwnedRecipeLimits,
    pub mapping: OwnedMappingLimits,
}
impl Default for WeaponProfileLimits {
    fn default() -> Self {
        Self {
            max_catalog_bytes: 8 * 1024 * 1024,
            max_base_catalog_bytes: 8 * 1024 * 1024,
            max_policy_bytes: 1024 * 1024,
            max_profiles: 8192,
            max_bases: 8192,
            max_fields: 128,
            max_work: 4_000_000,
            recipe: OwnedRecipeLimits::default(),
            mapping: OwnedMappingLimits::default(),
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum WeaponProfileError {
    #[error("weapon profile exceeds or has invalid limit: {0}")]
    Limit(&'static str),
    #[error("invalid weapon profile catalog or policy: {0}")]
    Invalid(&'static str),
    #[error("weapon profile artifact, source or predecessor binding differs")]
    Binding,
    #[error("weapon profile baseline preservation: {0}")]
    Preservation(String),
    #[error(transparent)]
    Mapping(#[from] OwnedMappingError),
    #[error(transparent)]
    Recipe(#[from] OwnedRecipeError),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
type Result<T> = std::result::Result<T, WeaponProfileError>;
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponProfileReceipt {
    pub catalog_sha256: String,
    pub base_catalog_sha256: String,
    pub policy: OwnedContentDigest,
    pub mapping: OwnedContentDigest,
    pub before_definitions: DataIdentity,
    pub after_definitions: DataIdentity,
    pub converted_profiles: usize,
    pub source_fields: usize,
    pub literal_defaults: usize,
    pub omitted_fields: usize,
    pub presence_facts: usize,
    pub changed_program_owners: usize,
    pub work_used: usize,
}
#[derive(Clone, Debug)]
pub struct StagedWeaponProfileRecipe {
    pub successor: OwnedRecipeInput,
    pub receipt: WeaponProfileReceipt,
}
fn unique_fields<'de, D: Deserializer<'de>>(
    d: D,
) -> std::result::Result<BTreeMap<String, f64>, D::Error> {
    struct Fields;
    impl<'de> Visitor<'de> for Fields {
        type Value = BTreeMap<String, f64>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("unique finite raw weapon fields")
        }
        fn visit_map<M: MapAccess<'de>>(
            self,
            mut map: M,
        ) -> std::result::Result<Self::Value, M::Error> {
            let mut fields = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, f64>()? {
                if !value.is_finite() || fields.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom(
                        "duplicate or nonfinite weapon field",
                    ));
                }
            }
            Ok(fields)
        }
    }
    d.deserialize_map(Fields)
}
fn key(text: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(text).expect("compiler-owned key")
}
fn text(value: &str) -> bool {
    !value.is_empty() && value.trim_ascii() == value && !value.chars().any(char::is_control)
}
fn charge(left: &mut usize, n: usize) -> Result<()> {
    *left = left
        .checked_sub(n)
        .ok_or(WeaponProfileError::Limit("work"))?;
    Ok(())
}
fn selector(base: &str) -> ExternalSelector {
    ExternalSelector::Definition(ExternalOwnerSelector::ItemTemplate {
        base: SourceComponent::Text(base.to_owned()),
        prototype: SourceComponent::Missing,
        variant: SourceComponent::Missing,
    })
}

/// Compile all finite field bindings without evaluating item quality, modifiers,
/// equipment legality, action compatibility, overrides, DPS or source-language code.
pub fn compile_owned_weapon_profiles(
    base: &StagedOwnedRecipe,
    mapping: &OwnedMappingIndex,
    base_catalog_bytes: &[u8],
    catalog_bytes: &[u8],
    policy: &WeaponProfilePolicy,
    limits: WeaponProfileLimits,
) -> Result<StagedWeaponProfileRecipe> {
    let hard = WeaponProfileLimits::default();
    for (name, value, ceiling) in [
        (
            "catalog bytes",
            limits.max_catalog_bytes,
            hard.max_catalog_bytes,
        ),
        (
            "base catalog bytes",
            limits.max_base_catalog_bytes,
            hard.max_base_catalog_bytes,
        ),
        (
            "policy bytes",
            limits.max_policy_bytes,
            hard.max_policy_bytes,
        ),
        ("profiles", limits.max_profiles, hard.max_profiles),
        ("bases", limits.max_bases, hard.max_bases),
        ("fields", limits.max_fields, hard.max_fields),
        ("work", limits.max_work, hard.max_work),
    ] {
        if value == 0 || value > ceiling {
            return Err(WeaponProfileError::Limit(name));
        }
    }
    if catalog_bytes.len() > limits.max_catalog_bytes
        || base_catalog_bytes.len() > limits.max_base_catalog_bytes
    {
        return Err(WeaponProfileError::Limit("catalog bytes"));
    }
    let policy_digest = digest_owned(
        "owned-weapon-profile-policy-v1",
        policy,
        limits.max_policy_bytes,
    )?;
    let catalog_sha256 = format!("{:x}", Sha256::digest(catalog_bytes));
    let base_catalog_sha256 = format!("{:x}", Sha256::digest(base_catalog_bytes));
    if catalog_sha256 != policy.catalog_sha256 || base_catalog_sha256 != policy.base_catalog_sha256
    {
        return Err(WeaponProfileError::Binding);
    }
    let catalog: WeaponProfileCatalog = serde_json::from_slice(catalog_bytes)?;
    let base_catalog: ItemBaseCatalog = serde_json::from_slice(base_catalog_bytes)?;
    if catalog.schema_version != OWNED_WEAPON_PROFILE_VERSION
        || policy.schema_version != OWNED_WEAPON_PROFILE_VERSION
        || base_catalog.schema_version != OWNED_ITEM_BASE_VERSION
    {
        return Err(WeaponProfileError::Invalid("version"));
    }
    if catalog.profiles.is_empty()
        || catalog.profiles.len() > limits.max_profiles
        || base_catalog.bases.is_empty()
        || base_catalog.bases.len() > limits.max_bases
    {
        return Err(WeaponProfileError::Limit("profiles or bases"));
    }
    if policy.fields.is_empty() || policy.fields.len() > limits.max_fields {
        return Err(WeaponProfileError::Limit("fields"));
    }
    mapping.validate_limits(limits.mapping)?;
    if mapping.input().registry != base.registry().identity()?
        || mapping.input().definitions != *base.schema().identity()
        || catalog.base_catalog_sha256 != base_catalog_sha256
        || catalog.source != base_catalog.source
        || catalog.source.system != ExternalSourceSystem::PathOfBuilding2
        || catalog.source.system != mapping.input().source.system
        || catalog.source.revision != mapping.input().source.revision
        || catalog.source.files.is_empty()
    {
        return Err(WeaponProfileError::Binding);
    }
    let mut left = limits.max_work;
    charge(
        &mut left,
        catalog.profiles.len() + base_catalog.bases.len() + policy.fields.len(),
    )?;
    let mut pins = BTreeSet::new();
    for pin in &catalog.source.files {
        charge(&mut left, mapping.input().source.files.len() + 1)?;
        if !pins.insert(pin.path.as_str()) || !mapping.input().source.files.contains(pin) {
            return Err(WeaponProfileError::Binding);
        }
    }
    let mut fields = BTreeMap::new();
    let mut stats = BTreeSet::new();
    let mut capabilities = BTreeSet::new();
    for field in &policy.fields {
        if !text(&field.source_field)
            || !stats.insert(field.stat.clone())
            || fields.insert(field.source_field.as_str(), field).is_some()
        {
            return Err(WeaponProfileError::Invalid(
                "duplicate or invalid field/stat",
            ));
        }
        let SchemaLookup::Known(stat) = base.schema().definition(&field.stat) else {
            return Err(WeaponProfileError::Invalid("unknown target stat"));
        };
        if stat.targets != [RuleEntityKind::EquipmentUse]
            || stat.value
                != (ComputedValueType::Quantity {
                    unit: field.unit.clone(),
                })
            || !matches!(
                base.schema().definition(&field.unit),
                SchemaLookup::Known(_)
            )
        {
            return Err(WeaponProfileError::Invalid(
                "target stat type, unit or scope",
            ));
        }
        if let WeaponFieldAbsence::Literal(value) = &field.when_absent
            && !matches!(value, ParameterValue::Quantity(q) if q.unit() == &field.unit)
        {
            return Err(WeaponProfileError::Invalid(
                "absence literal must have exact output unit",
            ));
        }
        if let Some(id) = &field.presence {
            let SchemaLookup::Known(schema) = base.schema().definition(id) else {
                return Err(WeaponProfileError::Invalid("unknown presence capability"));
            };
            if schema.targets != [RuleEntityKind::EquipmentUse] || !capabilities.insert(id.clone())
            {
                return Err(WeaponProfileError::Invalid(
                    "presence capability scope or duplicate",
                ));
            }
        }
    }
    let mut expected = BTreeMap::new();
    let mut base_names = BTreeSet::new();
    let mut template_ids = BTreeSet::new();
    for row in &base_catalog.bases {
        if !text(&row.name)
            || !text(&row.item_type)
            || !pins.contains(row.source_module.as_str())
            || !base_names.insert(row.name.as_str())
        {
            return Err(WeaponProfileError::Invalid(
                "duplicate, invalid or unpinned base",
            ));
        }
        let Some(MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::ItemTemplate(id)),
            basis: MappingBasis::Exact,
        }) = mapping.lookup(&selector(&row.name))
        else {
            return Err(WeaponProfileError::Invalid(
                "missing or unresolved exact base identity",
            ));
        };
        if !matches!(base.schema().definition(id), SchemaLookup::Known(_))
            || !template_ids.insert(id.clone())
        {
            return Err(WeaponProfileError::Invalid("unknown or aliased template"));
        }
        if row.weapon_field == ItemBaseWeaponField::Table {
            expected.insert(row.name.as_str(), id.clone());
        }
    }
    let mut rows = BTreeMap::new();
    let mut source_fields = 0;
    let mut observed = BTreeSet::new();
    for row in &catalog.profiles {
        if !text(&row.base) || rows.insert(row.base.as_str(), row).is_some() {
            return Err(WeaponProfileError::Invalid(
                "duplicate or invalid profile base",
            ));
        }
        if row.fields.is_empty() || row.fields.len() > limits.max_fields {
            return Err(WeaponProfileError::Limit("profile fields"));
        }
        charge(&mut left, row.fields.len())?;
        for (name, value) in &row.fields {
            if !value.is_finite() || !fields.contains_key(name.as_str()) {
                return Err(WeaponProfileError::Invalid(
                    "nonfinite or unreviewed source field",
                ));
            }
            observed.insert(name.as_str());
        }
        source_fields += row.fields.len();
    }
    if rows.keys().copied().collect::<Vec<_>>() != expected.keys().copied().collect::<Vec<_>>() {
        return Err(WeaponProfileError::Invalid(
            "complete profile membership differs from known table bases",
        ));
    }
    if observed != fields.keys().copied().collect() {
        return Err(WeaponProfileError::Invalid(
            "policy field membership differs from source",
        ));
    }
    let (mut literal_defaults, mut omitted_fields, mut presence_facts) = (0, 0, 0);
    let mut programs = Vec::with_capacity(rows.len());
    for (name, row) in &rows {
        let mut program = RuleProgram {
            id: key("raw-base-attack-profile"),
            context: RuleEntityKind::EquipmentUse,
            reads: vec![],
            nodes: vec![],
            effects: vec![],
        };
        for (i, (source_field, field)) in fields.iter().enumerate() {
            charge(&mut left, 3)?;
            let source_value = row.fields.get(*source_field);
            let value = match source_value {
                Some(value) => Some(ParameterValue::Quantity(
                    FiniteQuantity::new(*value, field.unit.clone())
                        .map_err(|_| WeaponProfileError::Invalid("nonfinite source value"))?,
                )),
                None => match &field.when_absent {
                    WeaponFieldAbsence::Required => {
                        return Err(WeaponProfileError::Invalid("required profile field absent"));
                    }
                    WeaponFieldAbsence::Omit => {
                        omitted_fields += 1;
                        None
                    }
                    WeaponFieldAbsence::Literal(value) => {
                        literal_defaults += 1;
                        Some(value.clone())
                    }
                },
            };
            if let Some(value) = value {
                let value_key = key(&format!("value-{i}"));
                program.nodes.push(RuleNode {
                    id: value_key.clone(),
                    expression: RuleExpression::Literal { value },
                });
                program.effects.push(RuleEffect {
                    id: key(&format!("raw-{i}")),
                    when: None,
                    effect: RuleEffectKind::Derive {
                        entity: RuleEntity::Current,
                        stat: field.stat.clone(),
                        value: value_key,
                    },
                });
            }
            if let Some(capability) = &field.presence {
                presence_facts += 1;
                let value_key = key(&format!("present-{i}"));
                program.nodes.push(RuleNode {
                    id: value_key.clone(),
                    expression: RuleExpression::Literal {
                        value: ParameterValue::Boolean(source_value.is_some()),
                    },
                });
                program.effects.push(RuleEffect {
                    id: key(&format!("field-present-{i}")),
                    when: None,
                    effect: RuleEffectKind::Capability {
                        entity: RuleEntity::Current,
                        capability: capability.clone(),
                        enabled: value_key,
                    },
                });
            }
        }
        programs.push((SchemaSubject::Definition(expected[name].address()), program));
    }
    // Shared exclusive writer/coverage append helper is supplied by the host
    // integration checkpoint; no source-specific mutation algorithm belongs here.
    let (successor, changed_program_owners) = crate::owned_baseline::append_exclusive_baselines(
        base,
        &programs,
        &stats,
        &capabilities,
        &mut left,
        limits.recipe,
    )
    .map_err(|error| match error {
        crate::owned_baseline::BaselineAppendError::Limit => WeaponProfileError::Limit("work"),
        crate::owned_baseline::BaselineAppendError::Preservation => {
            WeaponProfileError::Preservation(
                "existing owners, coverage or exclusive writers differ".into(),
            )
        }
        crate::owned_baseline::BaselineAppendError::Recipe(error) => {
            WeaponProfileError::Recipe(error)
        }
    })?;
    Ok(StagedWeaponProfileRecipe {
        receipt: WeaponProfileReceipt {
            catalog_sha256,
            base_catalog_sha256,
            policy: policy_digest,
            mapping: *mapping.identity(),
            before_definitions: base.schema().identity().clone(),
            after_definitions: successor.rules.definitions.clone(),
            converted_profiles: rows.len(),
            source_fields,
            literal_defaults,
            omitted_fields,
            presence_facts,
            changed_program_owners,
            work_used: limits.max_work - left,
        },
        successor,
    })
}
