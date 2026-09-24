//! Explicit metadata declarations with bounded, catalog-independent lookup costs.
use super::*;

pub(super) fn validate(
    input: &ItemSourceLayoutPolicyInput,
    known: &BTreeMap<&OwnedDefinitionKey, (usize, &ItemLineRule)>,
    roles: &BTreeMap<OwnedDefinitionKey, ItemRuleSourceRole>,
    text: &mut usize,
    work: &mut usize,
) -> Result<BTreeSet<OwnedDefinitionKey>> {
    let declarations = input.dialect.metadata_rules();
    if declarations.is_empty() {
        // Preserve the zero-work empty metadata path used by older dialects.
        return Ok(BTreeSet::new());
    }
    // Both maps are already ordered and bounded. Charge their complete borrowed
    // indexes before allocation, then use explicit binary searches rather than
    // charging a full catalog traversal for every metadata declaration.
    charge(work, known.len().saturating_add(roles.len()), "schema work")?;
    let known_index: Vec<_> = known.iter().map(|(key, value)| (*key, *value)).collect();
    let role_index: Vec<_> = roles.iter().map(|(key, value)| (key, *value)).collect();
    let comparisons = |len: usize| len.checked_ilog2().map_or(0, |bits| bits as usize + 2);
    let mut metadata_rules = BTreeSet::new();
    for id in declarations {
        charge(text, id.as_str().len(), "policy text")?;
        // Include both binary searches and a conservative insertion bound for
        // the declaration set; variable-length key comparisons remain charged.
        charge(
            work,
            id.as_str().len().saturating_add(1).saturating_mul(
                comparisons(known_index.len())
                    .saturating_add(comparisons(role_index.len()))
                    .saturating_add(metadata_rules.len())
                    .saturating_add(3),
            ),
            "schema work",
        )?;
        let role = role_index
            .binary_search_by(|(key, _)| (*key).cmp(id))
            .ok()
            .map(|index| role_index[index].1);
        if !metadata_rules.insert(id.clone()) || role != Some(ItemRuleSourceRole::Header) {
            return Err(ItemSourceError::Policy(
                "unknown, duplicate or non-header metadata rule",
            ));
        }
        let index = known_index
            .binary_search_by(|(key, _)| (*key).cmp(id))
            .map_err(|_| ItemSourceError::Policy("unknown metadata rule"))?;
        let rule = known_index[index].1.1;
        charge(work, rule.emissions.len().saturating_add(1), "schema work")?;
        if rule.emissions.is_empty()
            || !rule
                .emissions
                .iter()
                .all(|emission| matches!(emission, ItemEmission::Metadata { .. }))
        {
            return Err(ItemSourceError::Policy(
                "metadata rule must emit only explicit metadata",
            ));
        }
    }
    Ok(metadata_rules)
}
