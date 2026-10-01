//! Finite source categories become ordinary owned Option inputs only after proof.
use super::*;
use poe_optimizer_core::{owned_definitions::OptionDefId, owned_schema::ValueSchema};

pub(super) type LineOptions = BTreeMap<usize, BTreeMap<OwnedDefinitionKey, OptionDefId>>;
#[derive(Clone, Debug, Default)]
pub(super) struct PreparedCategories {
    values: BTreeMap<OwnedDefinitionKey, BTreeMap<SourceModifierCategory, OptionDefId>>,
    rules: BTreeMap<OwnedDefinitionKey, BTreeSet<OwnedDefinitionKey>>,
    pub schema_work: usize,
    pub reference_text: usize,
}

pub(super) fn charge_shape(
    input: &ItemSourceLayoutPolicyInput,
    limits: ItemSourceLimits,
    text: &mut usize,
) -> Result<()> {
    let bindings = input.dialect.category_bindings();
    if bindings.len() > limits.max_properties {
        return Err(ItemSourceError::Limit("category inputs"));
    }
    for binding in bindings {
        charge(text, binding.input.as_str().len(), "policy text")?;
        // This is a source-format enum, not an extensible collection of game IDs.
        if binding.values.is_empty() || binding.values.len() > 4 {
            return Err(ItemSourceError::Policy("category value count"));
        }
        for value in &binding.values {
            charge(text, serde_json::to_vec(value)?.len(), "policy text")?;
        }
    }
    Ok(())
}

pub(super) fn validate<I: DefinitionSchemaIndex>(
    input: &ItemSourceLayoutPolicyInput,
    lines: &OwnedItemLinePolicy,
    schema: &I,
    boolean_inputs: &BTreeSet<OwnedDefinitionKey>,
    limits: ItemSourceLimits,
    text: &mut usize,
    work: &mut usize,
) -> Result<PreparedCategories> {
    let versioned = matches!(
        input.dialect,
        ItemSourceDialect::PobExportedSingleTextCategoriesV1 { .. }
    );
    // Preserve historical budget accounting. ContextOption did not exist in
    // the legacy line dialects, and is never supplied by old source policies.
    if !versioned && lines.input().schema_version < OWNED_ITEM_LINE_POLICY_V7 {
        return Ok(PreparedCategories::default());
    }
    let before = *work;
    charge_shape(input, limits, text)?;
    let mut prepared = PreparedCategories::default();
    for binding in input.dialect.category_bindings() {
        charge(
            work,
            binding.input.as_str().len().saturating_add(1),
            "schema work",
        )?;
        if boolean_inputs.contains(&binding.input) || prepared.values.contains_key(&binding.input) {
            return Err(ItemSourceError::Policy(
                "duplicate or conflicting category input",
            ));
        }
        let mut values = BTreeMap::new();
        let mut targets = BTreeSet::new();
        for row in &binding.values {
            charge(work, 1, "schema work")?;
            if row.value.namespace() != &input.namespace
                || !matches!(schema.definition(&row.value), SchemaLookup::Known(_))
                || values.insert(row.category, row.value.clone()).is_some()
                || !targets.insert(row.value.clone())
            {
                return Err(ItemSourceError::Policy(
                    "invalid or duplicate category value",
                ));
            }
        }
        prepared.values.insert(binding.input.clone(), values);
    }
    let mut used = BTreeSet::new();
    for rule in &lines.input().rules {
        charge(work, 1, "schema work")?;
        let mut required = BTreeSet::new();
        for emission in &rule.emissions {
            charge(work, 1, "schema work")?;
            let ItemEmission::Modifier { rolls, .. } = emission else {
                continue;
            };
            for roll in rolls {
                charge(work, 1, "schema work")?;
                let ItemLineValue::ContextOption { input: name } = &roll.value else {
                    continue;
                };
                charge(text, name.as_str().len(), "policy text")?;
                prepared.reference_text += name.as_str().len();
                let values = prepared
                    .values
                    .get(name)
                    .ok_or(ItemSourceError::Policy("unbound modifier option input"))?;
                let SchemaLookup::Known(slot) = schema.slot(&roll.slot) else {
                    return Err(ItemSourceError::Policy(
                        "category input requires a known slot",
                    ));
                };
                let ValueSchema::Option { allowed } = &slot.value else {
                    return Err(ItemSourceError::Policy(
                        "category input requires an Option slot",
                    ));
                };
                for value in values.values() {
                    charge(work, allowed.members.len().saturating_add(1), "schema work")?;
                    if !allowed.members.contains(value) {
                        return Err(ItemSourceError::Policy(
                            "category option not allowed by consuming slot",
                        ));
                    }
                }
                required.insert(name.clone());
                used.insert(name.clone());
            }
        }
        if !required.is_empty() {
            prepared.rules.insert(rule.id.clone(), required);
        }
    }
    if used.len() != prepared.values.len() {
        return Err(ItemSourceError::Policy("unused category input"));
    }
    prepared.schema_work = before - *work;
    Ok(prepared)
}

impl PreparedCategories {
    pub(super) fn project(
        &self,
        report: &ItemAttributionReport,
        work: &mut usize,
        output: &mut usize,
    ) -> Result<LineOptions> {
        let mut inputs = LineOptions::new();
        // Pending reports contain useful diagnostic positions, including
        // positions that omitted unconverted preceding members. They are not
        // proof of category, even when the current physical line is recognized.
        if self.values.is_empty() || !matches!(report.layout, ItemLayoutStatus::Proven) {
            return Ok(inputs);
        }
        for line in &report.lines {
            charge(work, 1, "work")?;
            let (Some(rule), Some(member)) = (&line.rule, line.member) else {
                continue;
            };
            if !line.blockers.is_empty() || line.presentation {
                continue;
            }
            charge(
                work,
                rule.as_str()
                    .len()
                    .saturating_add(1)
                    .saturating_mul(self.rules.len().max(1)),
                "work",
            )?;
            let Some(required) = self.rules.get(rule) else {
                continue;
            };
            let mut row = BTreeMap::new();
            for name in required {
                charge(
                    work,
                    name.as_str()
                        .len()
                        .saturating_add(1)
                        .saturating_mul(self.values.len().max(1)),
                    "work",
                )?;
                let values = &self.values[name];
                charge(work, values.len(), "work")?;
                if let Some(value) = values.get(&member.category) {
                    // Charge before cloning retained typed values. An unmapped
                    // category stays missing, never falls back to Explicit.
                    charge(output, 1, "output records")?;
                    charge(
                        work,
                        serde_json::to_vec(value)?
                            .len()
                            .saturating_add(name.as_str().len()),
                        "work",
                    )?;
                    row.insert(name.clone(), value.clone());
                }
            }
            if !row.is_empty() {
                charge(output, 1, "output records")?;
                inputs.insert(line.index, row);
            }
        }
        Ok(inputs)
    }
}
