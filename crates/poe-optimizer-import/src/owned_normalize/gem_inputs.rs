//! Injected physical-gem input recipes. An empty declaration is not source proof.
use super::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GemInputPolicy {
    pub definitions: DataIdentity,
    pub gems: Vec<GemInputRule>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GemInputRule {
    pub gem: GemDefId,
    /// Exact reviewed source values; Missing is explicit and never a wildcard.
    pub guards: Vec<GemInputGuard>,
    pub parameters: Vec<GemParameterInput>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GemInputGuard {
    pub attribute: String,
    pub allowed: Vec<SourceComponent>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GemParameterInput {
    pub slot: DeclaredSlot<ParameterSlotDefId>,
    pub value: ValueRecipeInput,
}

pub(super) struct CompiledGemInputs {
    rules: BTreeMap<GemDefId, CompiledRule>,
    pub work: usize,
}
struct CompiledRule {
    guards: Vec<GemInputGuard>,
    parameters: Vec<CompiledParameter>,
    complete: bool,
}
struct CompiledParameter {
    slot: DeclaredSlot<ParameterSlotDefId>,
    schema: ParameterSlotSchema,
    recipe: ValueRecipe,
}

fn charge(work: &mut usize, amount: usize, limits: NormalizationLimits) -> Result<()> {
    *work = work
        .checked_add(amount)
        .filter(|work| *work <= limits.max_work)
        .ok_or(NormalizationError::Limit("gem input schema work"))?;
    Ok(())
}
fn value_valid(value: &ParameterValue, schema: &ValueSchema) -> bool {
    match (value, schema) {
        (ParameterValue::Boolean(_), ValueSchema::Boolean) => true,
        (ParameterValue::Integer(value), ValueSchema::Integer(range)) => {
            *value >= range.minimum && *value <= range.maximum
        }
        (ParameterValue::Quantity(value), ValueSchema::Quantity(range)) => {
            value.unit() == range.minimum.unit()
                && value.unit() == range.maximum.unit()
                && value.value() >= range.minimum.value()
                && value.value() <= range.maximum.value()
        }
        (ParameterValue::Option(value), ValueSchema::Option { allowed }) => {
            allowed.members.contains(value)
        }
        _ => false,
    }
}
fn known<I: DefinitionSchemaIndex, D: SchemaDefinitionId>(definitions: &I, id: &D) -> bool {
    matches!(definitions.definition(id), SchemaLookup::Known(_))
}

pub(super) fn compile<I: DefinitionSchemaIndex>(
    policy: Option<&GemInputPolicy>,
    definitions: &I,
    limits: NormalizationLimits,
) -> Result<Option<CompiledGemInputs>> {
    let Some(policy) = policy else {
        return Ok(None);
    };
    if &policy.definitions != definitions.identity() {
        return Err(NormalizationError::Binding);
    }
    if policy.gems.len() > 4096 {
        return Err(NormalizationError::Policy("gem input rules"));
    }
    let mut rules = BTreeMap::new();
    let mut work = 0;
    for rule in &policy.gems {
        charge(&mut work, 1, limits)?;
        if rule.guards.len() > 64 || rule.parameters.len() > 64 || rules.contains_key(&rule.gem) {
            return Err(NormalizationError::Policy(
                "duplicate or oversized gem input rule",
            ));
        }
        let SchemaLookup::Known(gem) = definitions.definition(&rule.gem) else {
            return Err(NormalizationError::Policy("gem input owner schema"));
        };
        charge(&mut work, gem.declarations.parameters.members.len(), limits)?;
        let declared: BTreeSet<_> = gem.declarations.parameters.members.iter().collect();
        let mut guard_names = BTreeSet::new();
        for guard in &rule.guards {
            charge(
                &mut work,
                guard.allowed.len() + guard.attribute.len(),
                limits,
            )?;
            if guard.attribute.is_empty()
                || guard.attribute.len() > 128
                || guard.allowed.len() > 64
                || !guard_names.insert(&guard.attribute)
                || guard.allowed.iter().collect::<BTreeSet<_>>().len() != guard.allowed.len()
            {
                return Err(NormalizationError::Policy("gem input guards"));
            }
            for value in &guard.allowed {
                if let SourceComponent::Text(text) = value {
                    if text.len() > limits.mapping.max_string_bytes {
                        return Err(NormalizationError::Policy("gem input guard bytes"));
                    }
                    charge(&mut work, text.len(), limits)?;
                }
            }
        }
        let mut slots = BTreeSet::new();
        let mut parameters = Vec::new();
        for input in &rule.parameters {
            charge(&mut work, 1, limits)?;
            if input.slot.declaration != SlotOwnerDefId::Gem(rule.gem.clone())
                || !declared.contains(&input.slot)
                || !slots.insert(&input.slot)
            {
                return Err(NormalizationError::Policy("gem input slot declaration"));
            }
            let SchemaLookup::Known(schema) = definitions.slot(&input.slot) else {
                return Err(NormalizationError::Policy("gem input slot schema"));
            };
            charge(&mut work, schema.sites.len(), limits)?;
            if let ValueSchema::Option { allowed } = &schema.value {
                charge(&mut work, allowed.members.len(), limits)?;
            }
            if !schema.sites.contains(&ParameterSite::GemParameter)
                || &input.value.codec.namespace != definitions.namespace()
                || input
                    .value
                    .tiers
                    .iter()
                    .flat_map(|tier| &tier.selectors)
                    .any(|selector| selector.lane != ValueLane::Attribute)
            {
                return Err(NormalizationError::Policy(
                    "gem input slot site or selector",
                ));
            }
            let compatible = match (&input.value.codec.codec, &schema.value) {
                (ValueCodecKind::Boolean { .. }, ValueSchema::Boolean)
                | (ValueCodecKind::Integer { .. }, ValueSchema::Integer(_)) => true,
                (ValueCodecKind::Quantity { unit, .. }, ValueSchema::Quantity(range)) => {
                    unit == range.minimum.unit()
                        && unit == range.maximum.unit()
                        && known(definitions, unit)
                }
                (ValueCodecKind::Option { tokens }, ValueSchema::Option { allowed }) => {
                    charge(
                        &mut work,
                        tokens.len().saturating_mul(allowed.members.len()),
                        limits,
                    )?;
                    tokens.iter().all(|token| {
                        allowed.members.contains(&token.value) && known(definitions, &token.value)
                    })
                }
                _ => false,
            };
            if !compatible {
                return Err(NormalizationError::Policy("gem input codec schema"));
            }
            if let MissingValuePolicy::Explicit { value } = &input.value.missing
                && (!value_valid(value, &schema.value)
                    || matches!(value, ParameterValue::Option(option) if !known(definitions, option)))
            {
                return Err(NormalizationError::Policy("gem input default schema"));
            }
            charge(&mut work, input.value.numeric_aliases.len(), limits)?;
            let recipe = ValueRecipe::new(input.value.clone(), limits.value)?;
            if recipe
                .alias_values()
                .any(|value| !value_valid(value, &schema.value))
            {
                return Err(NormalizationError::Policy(
                    "gem input alias outside slot schema",
                ));
            }
            parameters.push(CompiledParameter {
                slot: input.slot.clone(),
                schema: schema.clone(),
                recipe,
            });
        }
        rules.insert(
            rule.gem.clone(),
            CompiledRule {
                guards: rule.guards.clone(),
                parameters,
                complete: gem.declarations.parameters.is_complete() && slots == declared,
            },
        );
    }
    Ok(Some(CompiledGemInputs { rules, work }))
}

impl Builder<'_, '_> {
    pub(super) fn gem_parameters(
        &mut self,
        row: &SourceEvidenceRow<'_>,
        gem: &DraftField<GemDefId>,
        policy: Option<&CompiledGemInputs>,
    ) -> Result<DraftList<ParameterDraft>> {
        let source = row.occurrence().id();
        self.charge(1)?;
        let rule = match (gem, policy) {
            (DraftField::Known { value }, Some(policy)) => policy.rules.get(value),
            _ => None,
        };
        let Some(rule) = rule else {
            return self.closure(source, "gem-parameters-not-converted", vec![]);
        };
        if row.occurrence().has_namespace_context() {
            return self.closure(source, "gem-parameters-not-converted", vec![]);
        }
        for guard in &rule.guards {
            self.charge(row.attributes().len() + guard.allowed.len())?;
            let mut attributes = row.attributes().iter().filter(|attribute| {
                attribute.origin().namespace.is_none() && attribute.origin().name == guard.attribute
            });
            let attribute = attributes.next();
            if attributes.next().is_some() {
                return self.closure(source, "gem-parameters-not-converted", vec![]);
            }
            let value = match attribute {
                None => SourceComponent::Missing,
                Some(attribute) => {
                    if attribute.raw().len() > self.limits.mapping.max_string_bytes {
                        return Err(NormalizationError::Limit("gem input guard bytes"));
                    }
                    let Ok(value) = attribute.decoded() else {
                        return self.closure(source, "gem-parameters-not-converted", vec![]);
                    };
                    self.charge(value.len().saturating_mul(guard.allowed.len().max(1)))?;
                    SourceComponent::Text(value.into())
                }
            };
            if !guard.allowed.contains(&value) {
                return self.closure(source, "gem-parameters-not-converted", vec![]);
            }
        }
        let mut members = Vec::new();
        let mut all_converted = rule.complete;
        for input in &rule.parameters {
            if let ValueSchema::Option { allowed } = &input.schema.value {
                self.charge(allowed.members.len())?;
            }
            for selector in input
                .recipe
                .input()
                .tiers
                .iter()
                .flat_map(|tier| &tier.selectors)
            {
                let bytes = self.attributes[source.ordinal() as usize]
                    .get(selector.name.as_str())
                    .map_or(0, |(_, attribute)| attribute.raw().len());
                self.charge(bytes)?;
            }
            let value = match self.scalar_value(row, &input.recipe) {
                Ok(ScalarValue::Selected(value) | ScalarValue::Defaulted(value))
                    if value_valid(&value, &input.schema.value) =>
                {
                    Some(value)
                }
                Ok(ScalarValue::Absent) if input.schema.presence == SlotPresence::OptionalOnce => {
                    None
                }
                Ok(_) | Err(NormalizationError::Value(ValuePolicyError::MultipleValues { .. })) => {
                    all_converted = false;
                    None
                }
                Err(error) => return Err(error),
            };
            if let Some(value) = value {
                members.push(
                    ParameterAssignment {
                        slot: input.slot.clone(),
                        value,
                    }
                    .into(),
                );
            }
        }
        if all_converted {
            Ok(complete(members))
        } else {
            self.closure(source, "gem-parameters-not-converted", members)
        }
    }
}
