//! Exact physical-gem quality inputs. No numeric or kind defaults are implicit.
use super::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum GemQualityPolicy {
    Unconverted,
    Attributes(Box<GemQualityPolicyInput>),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GemQualityPolicyInput {
    pub definitions: DataIdentity,
    pub amount: ValueRecipeInput,
    pub kind_attribute: String,
    /// Missing is an explicit reviewed default-kind rule, not a wildcard.
    pub kinds: Vec<GemQualityKindRule>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GemQualityKindRule {
    pub source: SourceComponent,
    pub kind: QualityDefId,
}

pub(super) struct CompiledGemQuality {
    recipe: ValueRecipe,
    kind_attribute: String,
    kinds: BTreeMap<SourceComponent, CompiledKind>,
}
struct CompiledKind {
    kind: QualityDefId,
    amount: Option<QuantityRange>,
}

pub(super) fn compile<I: DefinitionSchemaIndex>(
    policy: &GemQualityPolicy,
    definitions: &I,
    limits: NormalizationLimits,
) -> Result<Option<CompiledGemQuality>> {
    let GemQualityPolicy::Attributes(input) = policy else {
        return Ok(None);
    };
    if &input.definitions != definitions.identity()
        || &input.amount.codec.namespace != definitions.namespace()
    {
        return Err(NormalizationError::Binding);
    }
    if input.kind_attribute.is_empty()
        || input.kind_attribute.len() > limits.value.max_selector_bytes
        || input.kinds.len() > 64
    {
        return Err(NormalizationError::Policy("gem quality kind rules"));
    }
    if !matches!(input.amount.missing, MissingValuePolicy::Pending)
        || input
            .amount
            .tiers
            .iter()
            .flat_map(|tier| &tier.selectors)
            .any(|selector| selector.lane != ValueLane::Attribute)
    {
        return Err(NormalizationError::Policy(
            "gem quality needs explicit attribute amount",
        ));
    }
    let ValueCodecKind::Quantity { unit, .. } = &input.amount.codec.codec else {
        return Err(NormalizationError::Policy("gem quality amount codec"));
    };
    if unit.namespace() != definitions.namespace() {
        return Err(NormalizationError::Binding);
    }
    let unit_known = match definitions.definition(unit) {
        SchemaLookup::Known(_) => true,
        SchemaLookup::Missing | SchemaLookup::Unmapped(_) => false,
        SchemaLookup::NamespaceMismatch | SchemaLookup::InconsistentIndex => {
            return Err(NormalizationError::Binding);
        }
    };
    let mut kinds = BTreeMap::new();
    for rule in &input.kinds {
        if rule.kind.namespace() != definitions.namespace() {
            return Err(NormalizationError::Binding);
        }
        if matches!(&rule.source, SourceComponent::Text(text) if text.len() > limits.mapping.max_string_bytes)
            || kinds.contains_key(&rule.source)
        {
            return Err(NormalizationError::Policy(
                "duplicate or oversized gem quality kind",
            ));
        }
        let amount = match definitions.definition(&rule.kind) {
            SchemaLookup::Known(schema) => {
                if schema.amount.minimum.unit() != unit
                    || schema.amount.maximum.unit() != unit
                    || schema.amount.minimum.value() > schema.amount.maximum.value()
                {
                    return Err(NormalizationError::Policy(
                        "gem quality schema unit or range",
                    ));
                }
                unit_known.then(|| schema.amount.clone())
            }
            SchemaLookup::Missing | SchemaLookup::Unmapped(_) => None,
            SchemaLookup::NamespaceMismatch | SchemaLookup::InconsistentIndex => {
                return Err(NormalizationError::Binding);
            }
        };
        kinds.insert(
            rule.source.clone(),
            CompiledKind {
                kind: rule.kind.clone(),
                amount,
            },
        );
    }
    Ok(Some(CompiledGemQuality {
        recipe: ValueRecipe::new(input.amount.clone(), limits.value)?,
        kind_attribute: input.kind_attribute.clone(),
        kinds,
    }))
}

impl Builder<'_, '_> {
    fn quality_pending(&mut self, source: SourceOccurrenceId, code: &str) -> Result<DraftQuality> {
        Ok(DraftQuality::Pending(PendingValue {
            id: self.issue(source)?,
            code: key(code),
            candidates: vec![],
        }))
    }
    pub(super) fn gem_quality<I: DefinitionSchemaIndex>(
        &mut self,
        row: &SourceEvidenceRow<'_>,
        gem: &DraftField<GemDefId>,
        policy: Option<&CompiledGemQuality>,
        definitions: &I,
    ) -> Result<DraftQuality> {
        let source = row.occurrence().id();
        let Some(policy) = policy else {
            return self.quality(source);
        };
        let amount = match self.scalar_value(row, &policy.recipe) {
            Ok(ScalarValue::Selected(ParameterValue::Quantity(value))) => value,
            Ok(ScalarValue::Missing | ScalarValue::Absent) => {
                return self.quality_pending(source, "gem-quality-amount-missing");
            }
            Ok(ScalarValue::Unavailable) => {
                return self.quality_pending(source, "gem-quality-amount-unavailable");
            }
            Ok(ScalarValue::Malformed) => {
                return self.quality_pending(source, "gem-quality-amount-malformed");
            }
            Err(NormalizationError::Value(ValuePolicyError::MultipleValues { .. })) => {
                return self.quality_pending(source, "gem-quality-amount-ambiguous");
            }
            Err(error) => return Err(error),
            _ => {
                return Err(NormalizationError::Policy(
                    "gem quality amount must be explicit quantity",
                ));
            }
        };
        self.charge(1)?;
        let kind_attribute = self.attributes[source.ordinal() as usize]
            .get(policy.kind_attribute.as_str())
            .map(|(_, attribute)| *attribute);
        let source_kind = match kind_attribute {
            None => SourceComponent::Missing,
            Some(attribute) => match attribute.decoded() {
                Ok(text) => {
                    if text.len() > self.limits.mapping.max_string_bytes {
                        return Err(NormalizationError::Limit("gem quality kind bytes"));
                    }
                    self.charge(text.len())?;
                    SourceComponent::Text(text.into())
                }
                Err(_) => return self.quality_pending(source, "gem-quality-kind-unavailable"),
            },
        };
        let Some(kind) = policy.kinds.get(&source_kind) else {
            return self.quality_pending(source, "gem-quality-kind-unmapped");
        };
        let Some(range) = &kind.amount else {
            return self.quality_pending(source, "gem-quality-schema-unresolved");
        };
        if amount.unit() != range.minimum.unit()
            || amount.value() < range.minimum.value()
            || amount.value() > range.maximum.value()
        {
            return self.quality_pending(source, "gem-quality-amount-outside-schema");
        }
        let DraftField::Known { value: gem } = gem else {
            return self.quality_pending(source, "gem-quality-owner-unresolved");
        };
        match definitions.definition(gem) {
            SchemaLookup::Known(schema) => {
                if schema.quality.presence == QualityPresence::Forbidden {
                    return self.quality_pending(source, "gem-quality-forbidden");
                }
                self.charge(schema.quality.allowed_kinds.members.len())?;
                if !schema.quality.allowed_kinds.members.contains(&kind.kind) {
                    return self.quality_pending(
                        source,
                        if schema.quality.allowed_kinds.is_complete() {
                            "gem-quality-kind-not-allowed"
                        } else {
                            "gem-quality-owner-schema-unresolved"
                        },
                    );
                }
            }
            // Identity-only Gem schema cannot disprove a separately schema-bound
            // explicit quality input. Its input/rule coverage remains Unmapped.
            SchemaLookup::Missing | SchemaLookup::Unmapped(_) => {}
            SchemaLookup::NamespaceMismatch | SchemaLookup::InconsistentIndex => {
                return Err(NormalizationError::Binding);
            }
        }
        Ok(Some(QualitySelection {
            kind: kind.kind.clone(),
            amount,
        })
        .into())
    }
}
