//! Offline finite intrinsic attack facts to exclusive owned player baselines.
//!
//! An exact catalog hash authenticates bytes against the caller's policy, not
//! their derivation from carried source pins. Source acquisition remains an
//! optional offline boundary. No Lua, synthetic item or action activation here.
use crate::{owned_mapping::*, owned_recipe::*};
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

pub const OWNED_INTRINSIC_ATTACK_VERSION: u32 = 1;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntrinsicAttackCatalog {
    pub schema_version: u32,
    pub source: SourcePin,
    pub classes: Vec<IntrinsicAttackClass>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntrinsicAttackClass {
    pub class_key: String,
    #[serde(deserialize_with = "unique_fields")]
    pub fields: BTreeMap<String, IntrinsicSourceValue>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IntrinsicSourceValue {
    Number(f64),
    Text(String),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntrinsicAttackPolicy {
    pub schema_version: u32,
    pub version: OwnedDefinitionKey,
    /// SHA-256 of exact catalog bytes, including any final newline.
    pub catalog_sha256: String,
    pub excluded_classes: Vec<String>,
    /// Every non-output source field needs an explicitly reviewed literal.
    #[serde(deserialize_with = "unique_fields")]
    pub constants: BTreeMap<String, IntrinsicSourceValue>,
    pub fields: Vec<IntrinsicAttackField>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntrinsicAttackField {
    pub source_field: String,
    pub stat: StatDefId,
    pub unit: UnitDefId,
}
#[derive(Clone, Copy, Debug)]
pub struct IntrinsicAttackLimits {
    pub max_catalog_bytes: usize,
    pub max_policy_bytes: usize,
    pub max_classes: usize,
    pub max_fields: usize,
    pub max_work: usize,
    pub recipe: OwnedRecipeLimits,
    pub mapping: OwnedMappingLimits,
}
impl Default for IntrinsicAttackLimits {
    fn default() -> Self {
        Self {
            max_catalog_bytes: 8 * 1024 * 1024,
            max_policy_bytes: 1024 * 1024,
            max_classes: 256,
            max_fields: 64,
            max_work: 1_000_000,
            recipe: OwnedRecipeLimits::default(),
            mapping: OwnedMappingLimits::default(),
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum IntrinsicAttackError {
    #[error("intrinsic attack exceeds or has invalid limit: {0}")]
    Limit(&'static str),
    #[error("invalid intrinsic attack catalog or policy: {0}")]
    Invalid(&'static str),
    #[error("intrinsic attack artifact, provenance or predecessor binding differs")]
    Binding,
    #[error("intrinsic attack cannot replace prior coverage or exclusive stat writers")]
    Preservation,
    #[error(transparent)]
    Mapping(#[from] OwnedMappingError),
    #[error(transparent)]
    Recipe(#[from] OwnedRecipeError),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
type Result<T> = std::result::Result<T, IntrinsicAttackError>;
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntrinsicAttackReceipt {
    pub catalog_sha256: String,
    pub policy: OwnedContentDigest,
    pub mapping: OwnedContentDigest,
    pub before_definitions: DataIdentity,
    pub after_definitions: DataIdentity,
    pub converted_classes: usize,
    pub excluded_classes: usize,
    pub changed_program_owners: usize,
    pub work_used: usize,
}
#[derive(Clone, Debug)]
pub struct StagedIntrinsicAttackRecipe {
    pub successor: OwnedRecipeInput,
    pub receipt: IntrinsicAttackReceipt,
}
fn unique_fields<'de, D: Deserializer<'de>>(
    d: D,
) -> std::result::Result<BTreeMap<String, IntrinsicSourceValue>, D::Error> {
    struct Fields;
    impl<'de> Visitor<'de> for Fields {
        type Value = BTreeMap<String, IntrinsicSourceValue>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("unique intrinsic fields")
        }
        fn visit_map<M: MapAccess<'de>>(
            self,
            mut map: M,
        ) -> std::result::Result<Self::Value, M::Error> {
            let mut fields = BTreeMap::new();
            while let Some((k, v)) = map.next_entry::<String, IntrinsicSourceValue>()? {
                if fields.insert(k, v).is_some() {
                    return Err(serde::de::Error::custom("duplicate intrinsic field"));
                }
            }
            Ok(fields)
        }
    }
    d.deserialize_map(Fields)
}
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).expect("compiler-owned key")
}
fn selector(class_key: &str) -> ExternalSelector {
    ExternalSelector::Definition(ExternalOwnerSelector::Class {
        key: SourceComponent::Text(class_key.to_owned()),
    })
}
fn valid_class_key(value: &str) -> bool {
    value.parse::<u32>().is_ok_and(|v| v.to_string() == value)
}
fn program(values: &[(StatDefId, FiniteQuantity)]) -> RuleProgram {
    RuleProgram {
        id: key("intrinsic-attack-baseline"),
        context: RuleEntityKind::Actor,
        reads: vec![],
        nodes: values
            .iter()
            .enumerate()
            .map(|(i, (_, value))| RuleNode {
                id: key(&format!("value-{i}")),
                expression: RuleExpression::Literal {
                    value: ParameterValue::Quantity(value.clone()),
                },
            })
            .collect(),
        effects: values
            .iter()
            .enumerate()
            .map(|(i, (stat, _))| RuleEffect {
                id: key(&format!("baseline-{i}")),
                when: None,
                effect: RuleEffectKind::Derive {
                    entity: RuleEntity::Player,
                    stat: stat.clone(),
                    value: key(&format!("value-{i}")),
                },
            })
            .collect(),
    }
}

/// Append class facts without changing schema, routing, registry, receivers,
/// operation versions or owner coverage. Baseline IDs are exclusive to these
/// mutually exclusive selected-class programs, not ordinary contributions.
pub fn compile_owned_intrinsic_attack(
    base: &StagedOwnedRecipe,
    mapping: &OwnedMappingIndex,
    catalog_bytes: &[u8],
    policy: &IntrinsicAttackPolicy,
    limits: IntrinsicAttackLimits,
) -> Result<StagedIntrinsicAttackRecipe> {
    let hard = IntrinsicAttackLimits::default();
    for (name, value, ceiling) in [
        (
            "catalog bytes",
            limits.max_catalog_bytes,
            hard.max_catalog_bytes,
        ),
        (
            "policy bytes",
            limits.max_policy_bytes,
            hard.max_policy_bytes,
        ),
        ("classes", limits.max_classes, hard.max_classes),
        ("fields", limits.max_fields, hard.max_fields),
        ("work", limits.max_work, hard.max_work),
    ] {
        if value == 0 || value > ceiling {
            return Err(IntrinsicAttackError::Limit(name));
        }
    }
    if catalog_bytes.len() > limits.max_catalog_bytes {
        return Err(IntrinsicAttackError::Limit("catalog bytes"));
    }
    let policy_digest = digest_owned(
        "owned-intrinsic-attack-policy-v1",
        policy,
        limits.max_policy_bytes,
    )?;
    let catalog_sha256 = format!("{:x}", Sha256::digest(catalog_bytes));
    if catalog_sha256 != policy.catalog_sha256 {
        return Err(IntrinsicAttackError::Binding);
    }
    let catalog: IntrinsicAttackCatalog = serde_json::from_slice(catalog_bytes)?;
    if catalog.schema_version != OWNED_INTRINSIC_ATTACK_VERSION
        || policy.schema_version != OWNED_INTRINSIC_ATTACK_VERSION
    {
        return Err(IntrinsicAttackError::Invalid("version"));
    }
    if catalog.classes.is_empty()
        || catalog.classes.len() > limits.max_classes
        || policy.excluded_classes.len() > limits.max_classes
    {
        return Err(IntrinsicAttackError::Limit("classes"));
    }
    if policy.fields.is_empty() || policy.fields.len() + policy.constants.len() > limits.max_fields
    {
        return Err(IntrinsicAttackError::Limit("fields"));
    }
    mapping.validate_limits(limits.mapping)?;
    if mapping.input().registry != base.registry().identity()?
        || mapping.input().definitions != *base.schema().identity()
        || catalog.source.system != mapping.input().source.system
        || catalog.source.revision != mapping.input().source.revision
        || catalog.source.files.is_empty()
    {
        return Err(IntrinsicAttackError::Binding);
    }
    let mut work = 0usize;
    let mut charge = |n: usize| -> Result<()> {
        work = work
            .checked_add(n)
            .filter(|v| *v <= limits.max_work)
            .ok_or(IntrinsicAttackError::Limit("work"))?;
        Ok(())
    };
    charge(
        catalog.classes.len()
            + policy.excluded_classes.len()
            + policy.fields.len()
            + policy.constants.len(),
    )?;
    let mut pins = BTreeSet::new();
    for pin in &catalog.source.files {
        charge(mapping.input().source.files.len() + 1)?;
        if !pins.insert(&pin.path) || !mapping.input().source.files.contains(pin) {
            return Err(IntrinsicAttackError::Binding);
        }
    }
    let mut excluded = BTreeSet::new();
    for value in &policy.excluded_classes {
        if !valid_class_key(value)
            || !excluded.insert(value.as_str())
            || matches!(
                mapping.lookup(&selector(value)),
                Some(MappingOutcome::Mapped { .. })
            )
        {
            return Err(IntrinsicAttackError::Invalid(
                "duplicate, invalid or mapped excluded class",
            ));
        }
    }
    let mut expected = BTreeSet::new();
    for (name, value) in &policy.constants {
        if name.is_empty() || matches!(value, IntrinsicSourceValue::Number(v) if !v.is_finite()) {
            return Err(IntrinsicAttackError::Invalid("constant"));
        }
        expected.insert(name.as_str());
    }
    let mut fields = BTreeMap::new();
    let mut stats = BTreeSet::new();
    for field in &policy.fields {
        if field.source_field.is_empty()
            || !expected.insert(field.source_field.as_str())
            || !stats.insert(field.stat.clone())
            || fields.insert(field.source_field.as_str(), field).is_some()
        {
            return Err(IntrinsicAttackError::Invalid(
                "duplicate output field, constant or stat",
            ));
        }
        let SchemaLookup::Known(stat) = base.schema().definition(&field.stat) else {
            return Err(IntrinsicAttackError::Invalid("unknown target stat"));
        };
        if stat.value
            != (ComputedValueType::Quantity {
                unit: field.unit.clone(),
            })
            || !stat.targets.contains(&RuleEntityKind::Actor)
            || !matches!(
                base.schema().definition(&field.unit),
                SchemaLookup::Known(_)
            )
        {
            return Err(IntrinsicAttackError::Invalid(
                "target stat type, unit or scope",
            ));
        }
    }
    let mut classes = BTreeMap::new();
    let mut seen_source = BTreeSet::new();
    let mut seen_excluded = BTreeSet::new();
    for row in &catalog.classes {
        charge(row.fields.len() + fields.len() + 1)?;
        if !valid_class_key(&row.class_key) || !seen_source.insert(row.class_key.as_str()) {
            return Err(IntrinsicAttackError::Invalid(
                "duplicate or invalid source class",
            ));
        }
        if row
            .fields
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
            != expected
            || policy
                .constants
                .iter()
                .any(|(k, v)| row.fields.get(k) != Some(v))
        {
            return Err(IntrinsicAttackError::Invalid(
                "unreviewed field membership or constant",
            ));
        }
        let mut values = Vec::with_capacity(fields.len());
        for (name, field) in &fields {
            let IntrinsicSourceValue::Number(value) = &row.fields[*name] else {
                return Err(IntrinsicAttackError::Invalid(
                    "numeric output field required",
                ));
            };
            let value = FiniteQuantity::new(*value, field.unit.clone())
                .map_err(|_| IntrinsicAttackError::Invalid("nonfinite output"))?;
            values.push((field.stat.clone(), value));
        }
        if excluded.contains(row.class_key.as_str()) {
            seen_excluded.insert(row.class_key.as_str());
            continue;
        }
        let Some(MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::Class(id)),
            ..
        }) = mapping.lookup(&selector(&row.class_key))
        else {
            return Err(IntrinsicAttackError::Invalid(
                "missing or unresolved class selector",
            ));
        };
        if !matches!(base.schema().definition(id), SchemaLookup::Known(_)) {
            return Err(IntrinsicAttackError::Invalid("unmapped class schema"));
        }
        if classes.insert(id.clone(), program(&values)).is_some() {
            return Err(IntrinsicAttackError::Invalid("aliased source class"));
        }
    }
    if seen_excluded != excluded {
        return Err(IntrinsicAttackError::Invalid(
            "excluded class missing from source",
        ));
    }
    charge(
        base.schema().input().definitions.len()
            + base.rules().input().owners.len()
            + base.rules().input().receivers.members.len(),
    )?;
    let class_ids: BTreeSet<_> = base
        .schema()
        .input()
        .definitions
        .iter()
        .filter_map(|d| {
            if let DefinitionDescriptor::Class(e) = d {
                Some(e.id.clone())
            } else {
                None
            }
        })
        .collect();
    if classes.is_empty() || classes.keys().cloned().collect::<BTreeSet<_>>() != class_ids {
        return Err(IntrinsicAttackError::Invalid(
            "complete class source membership differs",
        ));
    }
    // No receiver, contribution, child-actor projection, or unrelated producer
    // may share these exclusive baseline channels.
    if base
        .rules()
        .input()
        .receivers
        .members
        .iter()
        .any(|r| stats.contains(&r.stat))
    {
        return Err(IntrinsicAttackError::Preservation);
    }
    let mut rules = base.rules().input().clone();
    let mut changed_program_owners = 0;
    let mut seen_owners = BTreeSet::new();
    for row in &mut rules.owners {
        let desired = match &row.owner {
            SchemaSubject::Definition(DefinitionAddress::Class(id)) => classes.get(id),
            _ => None,
        };
        for prior in &row.programs.members {
            charge(prior.effects.len() + 1)?;
            if desired == Some(prior) {
                continue;
            }
            if desired.is_some_and(|p| p.id == prior.id)
                || prior.effects.iter().any(|e| match &e.effect {
                    RuleEffectKind::Derive { stat, .. }
                    | RuleEffectKind::Contribute { stat, .. }
                    | RuleEffectKind::ProjectActorStat { stat, .. } => stats.contains(stat),
                    _ => false,
                })
            {
                return Err(IntrinsicAttackError::Preservation);
            }
        }
        if let Some(desired) = desired {
            if !row.programs.members.contains(desired) {
                if row.programs.is_complete() {
                    return Err(IntrinsicAttackError::Preservation);
                }
                charge(desired.nodes.len() + desired.effects.len())?;
                row.programs.members.push(desired.clone());
                changed_program_owners += 1;
            }
            if let SchemaSubject::Definition(DefinitionAddress::Class(id)) = &row.owner {
                seen_owners.insert(id.clone());
            }
        }
    }
    if seen_owners != class_ids {
        return Err(IntrinsicAttackError::Invalid("missing class rule owner"));
    }
    let checked = assemble_owned_recipe(
        OwnedRecipeInput {
            schema_version: OWNED_RECIPE_VERSION,
            registry: base.registry().input().clone(),
            schema: base.schema().input().clone(),
            rules,
            routing: base.routing().input().clone(),
        },
        limits.recipe,
    )?;
    Ok(StagedIntrinsicAttackRecipe {
        successor: OwnedRecipeInput {
            schema_version: OWNED_RECIPE_VERSION,
            registry: checked.registry().input().clone(),
            schema: checked.schema().input().clone(),
            rules: checked.rules().input().clone(),
            routing: checked.routing().input().clone(),
        },
        receipt: IntrinsicAttackReceipt {
            catalog_sha256,
            policy: policy_digest,
            mapping: *mapping.identity(),
            before_definitions: base.schema().identity().clone(),
            after_definitions: checked.schema().identity().clone(),
            converted_classes: classes.len(),
            excluded_classes: seen_excluded.len(),
            changed_program_owners,
            work_used: work,
        },
    })
}
