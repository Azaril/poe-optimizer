//! Offline, source-pinned class scalar facts lowered to owned contributions.
//!
//! This adapter reads no build or Lua. Class game-rule closure is preserved:
//! base attributes do not cover class-specific unarmed defaults or every other
//! intrinsic mechanic. Empty input declarations have independent review scope.
use crate::{owned_mapping::*, owned_recipe::*};
use poe_optimizer_core::{
    data::DataIdentity,
    owned_build::ParameterValue,
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, SchemaPackageError};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{MapAccess, Visitor},
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

pub const OWNED_CLASS_BASES_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassBaseRecipePolicy {
    pub schema_version: u32,
    pub version: OwnedDefinitionKey,
    /// Exact source-pin path; this library never opens it.
    pub source_path: String,
    /// Complete, reviewed field membership of every source class record.
    pub expected_class_fields: Vec<String>,
    pub fields: Vec<ClassBaseFieldPolicy>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassBaseFieldPolicy {
    pub source_field: String,
    /// A pre-existing Actor Integer contribution channel, not a final total.
    pub stat: StatDefId,
}
#[derive(Clone, Copy, Debug)]
pub struct ClassBaseRecipeLimits {
    pub max_source_bytes: usize,
    pub max_policy_bytes: usize,
    pub max_classes: usize,
    pub max_fields: usize,
    pub max_work: usize,
    pub recipe: OwnedRecipeLimits,
    pub mapping: OwnedMappingLimits,
}
impl Default for ClassBaseRecipeLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: 8 * 1024 * 1024,
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
pub enum ClassBaseRecipeError {
    #[error("class base recipe exceeds or has invalid limit: {0}")]
    Limit(&'static str),
    #[error("invalid class base recipe: {0}")]
    Invalid(&'static str),
    #[error("class base source, mapping, registry or schema binding differs")]
    Binding,
    #[error("class base recipe cannot replace prior declarations or programs")]
    Preservation,
    #[error(transparent)]
    Mapping(#[from] OwnedMappingError),
    #[error(transparent)]
    Recipe(#[from] OwnedRecipeError),
    #[error(transparent)]
    Schema(#[from] SchemaPackageError),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
type Result<T> = std::result::Result<T, ClassBaseRecipeError>;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassBaseRecipeReceipt {
    pub source_sha256: String,
    pub policy: OwnedContentDigest,
    pub mapping: OwnedContentDigest,
    pub before_definitions: DataIdentity,
    pub after_definitions: DataIdentity,
    pub converted_classes: usize,
    pub refined_classes: usize,
    pub changed_program_owners: usize,
    pub work_used: usize,
}
#[derive(Clone, Debug)]
pub struct StagedClassBaseRecipe {
    pub successor: OwnedRecipeInput,
    /// Requires the caller's checked class declaration-refinement transition.
    pub refined: Vec<ClassDefId>,
    pub receipt: ClassBaseRecipeReceipt,
}

#[derive(Deserialize)]
struct SourceTree {
    classes: Vec<SourceClass>,
}
struct SourceClass(BTreeMap<String, Value>);
impl<'de> Deserialize<'de> for SourceClass {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct Fields;
        impl<'de> Visitor<'de> for Fields {
            type Value = SourceClass;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a class record with unique field names")
            }
            fn visit_map<M: MapAccess<'de>>(
                self,
                mut map: M,
            ) -> std::result::Result<Self::Value, M::Error> {
                let mut fields = BTreeMap::new();
                while let Some((k, v)) = map.next_entry::<String, Value>()? {
                    if fields.insert(k, v).is_some() {
                        return Err(serde::de::Error::custom("duplicate class field"));
                    }
                }
                Ok(SourceClass(fields))
            }
        }
        d.deserialize_map(Fields)
    }
}
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).expect("compiler-owned key")
}
fn close_empty<T>(set: &mut DeclaredSet<T>, owner: &SchemaSubject) -> Result<bool> {
    if !set.members.is_empty() {
        return Err(ClassBaseRecipeError::Preservation);
    }
    match &set.closure {
        SchemaClosure::Complete => Ok(false),
        SchemaClosure::Partial { gaps } => {
            if gaps.is_empty()
                || gaps.iter().any(|g| {
                    g.subject != *owner
                        || g.facet != SchemaFacet::InputSchema
                        || g.code != key("tree-declarations-not-converted")
                })
            {
                return Err(ClassBaseRecipeError::Preservation);
            }
            set.closure = SchemaClosure::Complete;
            Ok(true)
        }
    }
}
fn program(values: &[(StatDefId, BoundedInteger)]) -> RuleProgram {
    RuleProgram {
        id: key("class-base-contributions"),
        context: RuleEntityKind::Actor,
        reads: vec![],
        nodes: values
            .iter()
            .enumerate()
            .map(|(i, (_, value))| RuleNode {
                id: key(&format!("value-{i}")),
                expression: RuleExpression::Literal {
                    value: ParameterValue::Integer(*value),
                },
            })
            .collect(),
        effects: values
            .iter()
            .enumerate()
            .map(|(i, (stat, _))| RuleEffect {
                id: key(&format!("contribution-{i}")),
                when: None,
                effect: RuleEffectKind::Contribute {
                    entity: RuleEntity::Player,
                    stat: stat.clone(),
                    contribution: ContributionKind::Add,
                    value: key(&format!("value-{i}")),
                },
            })
            .collect(),
    }
}

/// Compile every source class to literal owned facts. Values and field-to-stat
/// assignments are data; no class names, base values or target IDs are built in.
/// Source hashes normalize CRLF exactly as the existing offline source pins do.
pub fn compile_owned_class_bases(
    base: &StagedOwnedRecipe,
    mapping: &OwnedMappingIndex,
    source_bytes: &[u8],
    policy: &ClassBaseRecipePolicy,
    limits: ClassBaseRecipeLimits,
) -> Result<StagedClassBaseRecipe> {
    let hard = ClassBaseRecipeLimits::default();
    for (name, value, ceiling) in [
        (
            "source bytes",
            limits.max_source_bytes,
            hard.max_source_bytes,
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
            return Err(ClassBaseRecipeError::Limit(name));
        }
    }
    if source_bytes.len() > limits.max_source_bytes {
        return Err(ClassBaseRecipeError::Limit("source bytes"));
    }
    let policy_digest = digest_owned(
        "owned-class-base-policy-v1",
        policy,
        limits.max_policy_bytes,
    )?;
    if policy.schema_version != OWNED_CLASS_BASES_VERSION
        || policy.source_path.is_empty()
        || policy.fields.is_empty()
        || policy.fields.len() > limits.max_fields
        || policy.expected_class_fields.len() > limits.max_fields
    {
        return Err(ClassBaseRecipeError::Invalid(
            "version, source path or field count",
        ));
    }
    mapping.validate_limits(limits.mapping)?;
    if mapping.input().registry != base.registry().identity()?
        || mapping.input().definitions != *base.schema().identity()
    {
        return Err(ClassBaseRecipeError::Binding);
    }
    let pin = mapping
        .input()
        .source
        .files
        .iter()
        .find(|p| p.path == policy.source_path)
        .ok_or(ClassBaseRecipeError::Binding)?;
    let normalized: Vec<u8> = source_bytes
        .iter()
        .enumerate()
        .filter(|(i, b)| **b != b'\r' || source_bytes.get(i + 1) != Some(&b'\n'))
        .map(|(_, b)| *b)
        .collect();
    let source_sha256 = format!("{:x}", Sha256::digest(&normalized));
    if pin.sha256 != source_sha256 {
        return Err(ClassBaseRecipeError::Binding);
    }
    let source: SourceTree = serde_json::from_slice(&normalized)?;
    if source.classes.is_empty() || source.classes.len() > limits.max_classes {
        return Err(ClassBaseRecipeError::Limit("classes"));
    }
    let mut expected = BTreeSet::new();
    for field in &policy.expected_class_fields {
        if field.is_empty() || !expected.insert(field.as_str()) {
            return Err(ClassBaseRecipeError::Invalid(
                "duplicate or empty reviewed field",
            ));
        }
    }
    if !expected.contains("integerId") {
        return Err(ClassBaseRecipeError::Invalid(
            "class identity field missing",
        ));
    }
    let mut fields = BTreeMap::new();
    let mut stats = BTreeSet::new();
    for field in &policy.fields {
        if field.source_field == "integerId"
            || !expected.contains(field.source_field.as_str())
            || fields
                .insert(field.source_field.as_str(), &field.stat)
                .is_some()
            || !stats.insert(&field.stat)
        {
            return Err(ClassBaseRecipeError::Invalid(
                "duplicate, absent or identity contribution field",
            ));
        }
        let SchemaLookup::Known(stat) = base.schema().definition(&field.stat) else {
            return Err(ClassBaseRecipeError::Invalid("unknown contribution stat"));
        };
        if stat.value != ComputedValueType::Integer
            || !stat.targets.contains(&RuleEntityKind::Actor)
        {
            return Err(ClassBaseRecipeError::Invalid(
                "contribution stat type or scope",
            ));
        }
    }
    let mut work = mapping.input().source.files.len()
        + policy.expected_class_fields.len()
        + policy.fields.len();
    let mut charge = |n: usize| -> Result<()> {
        work = work
            .checked_add(n)
            .filter(|v| *v <= limits.max_work)
            .ok_or(ClassBaseRecipeError::Limit("work"))?;
        Ok(())
    };
    charge(0)?;
    let mut classes = BTreeMap::new();
    let mut source_ids = BTreeSet::new();
    for SourceClass(row) in source.classes {
        charge(row.len() + fields.len() + 1)?;
        if row.keys().map(String::as_str).collect::<BTreeSet<_>>() != expected {
            return Err(ClassBaseRecipeError::Invalid(
                "unreviewed class field membership",
            ));
        }
        let source_id = row["integerId"]
            .as_u64()
            .filter(|v| *v > 0 && *v <= u32::MAX as u64)
            .ok_or(ClassBaseRecipeError::Invalid("class integer identity"))?;
        if !source_ids.insert(source_id) {
            return Err(ClassBaseRecipeError::Invalid("duplicate source class"));
        }
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::Class {
            key: SourceComponent::Text(source_id.to_string()),
        });
        let Some(MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::Class(id)),
            ..
        }) = mapping.lookup(&selector)
        else {
            return Err(ClassBaseRecipeError::Invalid(
                "missing or unresolved class selector",
            ));
        };
        let mut values = Vec::with_capacity(fields.len());
        for (field, stat) in &fields {
            let value = row[*field]
                .as_i64()
                .and_then(|v| BoundedInteger::new(v).ok())
                .ok_or(ClassBaseRecipeError::Invalid(
                    "class contribution is not a bounded integer",
                ))?;
            values.push(((*stat).clone(), value));
        }
        if classes.insert(id.clone(), program(&values)).is_some() {
            return Err(ClassBaseRecipeError::Invalid("aliased source class"));
        }
    }
    charge(base.schema().input().definitions.len() + base.rules().input().owners.len())?;
    let mut schema = base.schema().input().clone();
    let mut refined = Vec::new();
    let mut seen = BTreeSet::new();
    for descriptor in &mut schema.definitions {
        let DefinitionDescriptor::Class(entry) = descriptor else {
            continue;
        };
        if !classes.contains_key(&entry.id) {
            return Err(ClassBaseRecipeError::Invalid(
                "class source membership differs",
            ));
        }
        let SchemaState::Known(class) = &mut entry.schema else {
            return Err(ClassBaseRecipeError::Invalid("unmapped class schema"));
        };
        let owner = SchemaSubject::Definition(entry.id.address());
        let d = &mut class.declarations;
        charge(7)?;
        let changed = [
            close_empty(&mut d.parameters, &owner)?,
            close_empty(&mut d.choices, &owner)?,
            close_empty(&mut d.grants, &owner)?,
            close_empty(&mut d.actors, &owner)?,
            close_empty(&mut d.skill_grants, &owner)?,
            close_empty(&mut d.outputs, &owner)?,
            close_empty(&mut d.sockets, &owner)?,
        ]
        .into_iter()
        .any(|v| v);
        if changed {
            refined.push(entry.id.clone());
        }
        seen.insert(entry.id.clone());
    }
    if seen.len() != classes.len() {
        return Err(ClassBaseRecipeError::Invalid("missing class descriptor"));
    }
    let schema = OwnedDefinitionSchemaPackage::new(schema, limits.recipe.schema)?;
    let mut rules = base.rules().input().clone();
    let mut owners = BTreeSet::new();
    let mut changed_program_owners = 0;
    for row in &mut rules.owners {
        let SchemaSubject::Definition(DefinitionAddress::Class(id)) = &row.owner else {
            continue;
        };
        let desired = classes.get(id).ok_or(ClassBaseRecipeError::Invalid(
            "class rule membership differs",
        ))?;
        charge(row.programs.members.len() + desired.nodes.len() + desired.effects.len())?;
        if let Some(prior) = row.programs.members.iter().find(|p| p.id == desired.id) {
            if prior != desired {
                return Err(ClassBaseRecipeError::Preservation);
            }
        } else {
            if row.programs.is_complete() {
                return Err(ClassBaseRecipeError::Preservation);
            }
            row.programs.members.push(desired.clone());
            changed_program_owners += 1;
        }
        owners.insert(id.clone());
    }
    if owners.len() != classes.len() {
        return Err(ClassBaseRecipeError::Invalid("missing class rule owner"));
    }
    rules.definitions = schema.identity().clone();
    let mut routing = base.routing().input().clone();
    routing.definitions = schema.identity().clone();
    let successor = OwnedRecipeInput {
        schema_version: OWNED_RECIPE_VERSION,
        registry: base.registry().input().clone(),
        schema: schema.input().clone(),
        rules,
        routing,
    };
    let checked = assemble_owned_recipe(successor, limits.recipe)?;
    refined.sort();
    let receipt = ClassBaseRecipeReceipt {
        source_sha256,
        policy: policy_digest,
        mapping: *mapping.identity(),
        before_definitions: base.schema().identity().clone(),
        after_definitions: checked.schema().identity().clone(),
        converted_classes: classes.len(),
        refined_classes: refined.len(),
        changed_program_owners,
        work_used: work,
    };
    Ok(StagedClassBaseRecipe {
        successor: OwnedRecipeInput {
            schema_version: OWNED_RECIPE_VERSION,
            registry: checked.registry().input().clone(),
            schema: checked.schema().input().clone(),
            rules: checked.rules().input().clone(),
            routing: checked.routing().input().clone(),
        },
        refined,
        receipt,
    })
}
