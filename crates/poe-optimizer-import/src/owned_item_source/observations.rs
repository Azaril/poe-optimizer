//! Finite template applicability for derived preamble observations.
use super::*;

#[derive(Clone, Debug)]
pub(super) struct PreparedObservation {
    pub field: OwnedDefinitionKey,
    pub templates: Vec<ItemTemplateDefId>,
}

pub(super) fn charge_shape(
    input: &ItemSourceLayoutPolicyInput,
    limits: ItemSourceLimits,
    text: &mut usize,
) -> Result<()> {
    let mut remaining = limits.max_templates;
    for observation in input.dialect.preamble_observations() {
        charge(
            &mut remaining,
            observation.templates.len(),
            "observation templates",
        )?;
        charge(text, observation.rule.as_str().len(), "policy text")?;
        charge(text, observation.field.as_str().len(), "policy text")?;
        for template in &observation.templates {
            charge(text, template.key().as_str().len(), "policy text")?;
        }
    }
    Ok(())
}

pub(super) fn validate(
    input: &ItemSourceLayoutPolicyInput,
    known: &BTreeMap<&OwnedDefinitionKey, (usize, &ItemLineRule)>,
    roles: &BTreeMap<OwnedDefinitionKey, ItemRuleSourceRole>,
    prefixes: &BTreeMap<ItemTemplateDefId, ItemLoadIndexPrefix>,
    limits: ItemSourceLimits,
    text: &mut usize,
    work: &mut usize,
) -> Result<BTreeMap<OwnedDefinitionKey, PreparedObservation>> {
    charge_shape(input, limits, text)?;
    let declarations = input.dialect.preamble_observations();
    charge(work, declarations.len(), "schema work")?;
    if declarations.is_empty() {
        return Ok(BTreeMap::new());
    }
    charge(
        work,
        known
            .len()
            .saturating_add(roles.len())
            .saturating_add(prefixes.len()),
        "schema work",
    )?;
    let known_index: Vec<_> = known.iter().map(|(key, value)| (*key, *value)).collect();
    let role_index: Vec<_> = roles.iter().map(|(key, value)| (key, *value)).collect();
    // Namespace equality is checked once per index/reference. The merge can then
    // compare only keys, with exactly the same ordering as the complete IDs.
    let mut template_index = Vec::with_capacity(prefixes.len());
    for template in prefixes.keys() {
        validate_namespace(template, &input.namespace, work)?;
        template_index.push(template.key());
    }
    let comparisons = |len: usize| len.checked_ilog2().map_or(0, |bits| bits as usize + 2);
    let mut compiled = BTreeMap::new();
    let mut validated_templates: Vec<&[ItemTemplateDefId]> = Vec::new();
    for observation in declarations {
        charge(
            work,
            observation
                .rule
                .as_str()
                .len()
                .saturating_add(1)
                .saturating_mul(
                    comparisons(known_index.len())
                        .saturating_add(comparisons(role_index.len()))
                        .saturating_add(compiled.len())
                        .saturating_add(3),
                ),
            "schema work",
        )?;
        let role = role_index
            .binary_search_by(|(key, _)| (*key).cmp(&observation.rule))
            .ok()
            .map(|index| role_index[index].1);
        if role != Some(ItemRuleSourceRole::Unresolved)
            || compiled.contains_key(&observation.rule)
            || observation.templates.is_empty()
        {
            return Err(ItemSourceError::Policy(
                "observation needs a unique unresolved rule and nonempty templates",
            ));
        }
        let index = known_index
            .binary_search_by(|(key, _)| (*key).cmp(&observation.rule))
            .map_err(|_| ItemSourceError::Policy("unknown observation rule"))?;
        let rule = known_index[index].1.1;
        charge(work, rule.emissions.len().saturating_add(1), "schema work")?;
        if rule.emissions.is_empty()
            || !rule
                .emissions
                .iter()
                .all(|e| matches!(e, ItemEmission::Metadata { .. }))
        {
            return Err(ItemSourceError::Policy(
                "observation must emit only explicit metadata",
            ));
        }
        charge(work, observation.templates.len(), "schema work")?;
        let mut previously_validated = false;
        for validated in &validated_templates {
            if same_templates(validated, &observation.templates, work)? {
                previously_validated = true;
                break;
            }
        }
        if !previously_validated {
            let mut position = 0;
            let mut prior: Option<&OwnedDefinitionKey> = None;
            for template in &observation.templates {
                validate_namespace(template, &input.namespace, work)?;
                let key = template.key();
                if let Some(previous) = prior {
                    charge_comparison(previous, key, work)?;
                    if previous >= key {
                        return Err(ItemSourceError::Policy(
                            "observation templates must be strictly sorted and unique",
                        ));
                    }
                }
                prior = Some(key);
                loop {
                    let Some(known) = template_index.get(position) else {
                        return Err(ItemSourceError::Policy("unknown observation template"));
                    };
                    charge_comparison(known, key, work)?;
                    match known.cmp(&key) {
                        std::cmp::Ordering::Less => position += 1,
                        std::cmp::Ordering::Equal => {
                            position += 1;
                            break;
                        }
                        std::cmp::Ordering::Greater => {
                            return Err(ItemSourceError::Policy("unknown observation template"));
                        }
                    }
                }
            }
            validated_templates.push(&observation.templates);
        }
        compiled.insert(
            observation.rule.clone(),
            PreparedObservation {
                field: observation.field.clone(),
                templates: observation.templates.clone(),
            },
        );
    }
    Ok(compiled)
}

fn charge_comparison(
    left: &OwnedDefinitionKey,
    right: &OwnedDefinitionKey,
    work: &mut usize,
) -> Result<()> {
    charge(
        work,
        left.as_str()
            .len()
            .min(right.as_str().len())
            .saturating_add(1),
        "schema work",
    )
}

fn validate_namespace(
    template: &ItemTemplateDefId,
    namespace: &GameVersionNamespace,
    work: &mut usize,
) -> Result<()> {
    charge_comparison(template.namespace().game(), namespace.game(), work)?;
    charge_comparison(template.namespace().version(), namespace.version(), work)?;
    if template.namespace() != namespace {
        return Err(ItemSourceError::Policy(
            "observation template uses a foreign namespace",
        ));
    }
    Ok(())
}

// Repeated defence/alias declarations commonly have the same finite allowlist.
// Compare lengths first, then charge every full-ID equality before reusing an
// established proof. Shared prefixes or coincidentally equal keys are not proof.
fn same_templates(
    left: &[ItemTemplateDefId],
    right: &[ItemTemplateDefId],
    work: &mut usize,
) -> Result<bool> {
    charge(work, 1, "schema work")?;
    if left.len() != right.len() {
        return Ok(false);
    }
    for (left, right) in left.iter().zip(right) {
        charge_comparison(left.namespace().game(), right.namespace().game(), work)?;
        charge_comparison(
            left.namespace().version(),
            right.namespace().version(),
            work,
        )?;
        charge_comparison(left.key(), right.key(), work)?;
        if left != right {
            return Ok(false);
        }
    }
    Ok(true)
}
