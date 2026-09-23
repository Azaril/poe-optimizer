//! Bounded import-only source membership prerequisites. No source evaluation.
use super::*;
use crate::owned_value::ValueCodecKind;

#[derive(Clone, Debug)]
pub(super) struct PreparedCondition {
    rule_index: usize,
    member_index: usize,
}

pub(super) fn charge_text(input: &ItemSourceLayoutPolicyInput, text: &mut usize) -> Result<()> {
    for member in input.dialect.single_modifier_conditions() {
        charge(text, member.rule.as_str().len(), "policy text")?;
        for condition in &member.all {
            if let ItemSourceCondition::UnsignedIntegerCapture { capture, .. }
            | ItemSourceCondition::DecimalCapture { capture, .. } = condition
            {
                charge(text, capture.as_str().len(), "policy text")?;
            }
        }
    }
    Ok(())
}

pub(super) fn validate(
    input: &ItemSourceLayoutPolicyInput,
    known: &BTreeMap<&OwnedDefinitionKey, (usize, &ItemLineRule)>,
    roles: &BTreeMap<OwnedDefinitionKey, ItemRuleSourceRole>,
    text: &mut usize,
    work: &mut usize,
) -> Result<BTreeMap<OwnedDefinitionKey, PreparedCondition>> {
    let members = input.dialect.single_modifier_conditions();
    // Charge the full nested shape before any validation traversal or index allocation.
    charge(work, members.len(), "schema work")?;
    for member in members {
        charge(work, member.all.len(), "schema work")?;
    }
    charge_text(input, text)?;
    if members.is_empty() {
        return Ok(BTreeMap::new());
    }
    // Explicit sorted slices make the comparison bound independent of BTreeMap
    // internals. A linear map-size charge per lookup prevented ordinary catalogs
    // from validating even a few dozen conditional rules under the shared budget.
    charge(work, known.len().saturating_add(roles.len()), "schema work")?;
    let known_index: Vec<_> = known.iter().map(|(key, value)| (*key, *value)).collect();
    let role_index: Vec<_> = roles.iter().map(|(key, value)| (key, *value)).collect();
    let comparisons = |len: usize| len.checked_ilog2().map_or(0, |bits| bits as usize + 2);
    let mut compiled = BTreeMap::new();
    for (member_index, member) in members.iter().enumerate() {
        charge(
            work,
            member.rule.as_str().len().saturating_add(1).saturating_mul(
                comparisons(known_index.len())
                    .saturating_add(comparisons(role_index.len()))
                    .saturating_add(compiled.len())
                    .saturating_add(3),
            ),
            "schema work",
        )?;
        let role = role_index
            .binary_search_by(|(id, _)| (*id).cmp(&member.rule))
            .ok()
            .map(|index| role_index[index].1);
        if member.all.is_empty()
            || role != Some(ItemRuleSourceRole::Unresolved)
            || compiled.contains_key(&member.rule)
        {
            return Err(ItemSourceError::Policy(
                "conditional member needs a unique unresolved rule and nonempty prerequisites",
            ));
        }
        let index = known_index
            .binary_search_by(|(id, _)| (*id).cmp(&member.rule))
            .map_err(|_| ItemSourceError::Policy("unknown conditional member rule"))?;
        let (rule_index, rule) = known_index[index].1;
        charge(work, rule.emissions.len().saturating_add(1), "schema work")?;
        if rule.emissions.is_empty()
            || !rule
                .emissions
                .iter()
                .all(|e| matches!(e, ItemEmission::Modifier { .. }))
        {
            return Err(ItemSourceError::Policy(
                "conditional member must emit only modifiers",
            ));
        }
        let mut seen_tags = false;
        let mut seen_prefix = false;
        let mut seen_scaling_tags = false;
        let mut seen_initial_scaling = false;
        let mut captures = BTreeSet::new();
        for condition in &member.all {
            match condition {
                ItemSourceCondition::NoSourceTags => {
                    if std::mem::replace(&mut seen_tags, true) {
                        return Err(ItemSourceError::Policy("duplicate source-tag prerequisite"));
                    }
                }
                ItemSourceCondition::NoSourceScalingTags => {
                    if std::mem::replace(&mut seen_scaling_tags, true) {
                        return Err(ItemSourceError::Policy(
                            "duplicate scaling-tag prerequisite",
                        ));
                    }
                }
                ItemSourceCondition::InitialScalingIsOne => {
                    if std::mem::replace(&mut seen_initial_scaling, true) {
                        return Err(ItemSourceError::Policy(
                            "duplicate initial-scaling prerequisite",
                        ));
                    }
                }
                ItemSourceCondition::NoGeneratedBuffMembers => {
                    if std::mem::replace(&mut seen_prefix, true) {
                        return Err(ItemSourceError::Policy(
                            "duplicate generated-member prerequisite",
                        ));
                    }
                }
                ItemSourceCondition::UnsignedIntegerCapture { capture, .. }
                | ItemSourceCondition::DecimalCapture { capture, .. } => {
                    charge(
                        work,
                        capture.as_str().len().saturating_add(1).saturating_mul(
                            rule.captures
                                .len()
                                .saturating_add(captures.len())
                                .saturating_add(2),
                        ),
                        "schema work",
                    )?;
                    let invalid_bounds = match condition {
                        ItemSourceCondition::UnsignedIntegerCapture { min, max, .. } => min > max,
                        ItemSourceCondition::DecimalCapture { min, max, .. } => {
                            !min.is_finite() || !max.is_finite() || min > max
                        }
                        _ => unreachable!("numeric condition"),
                    };
                    if invalid_bounds || !captures.insert(capture) {
                        return Err(ItemSourceError::Policy(
                            "invalid or duplicate capture prerequisite",
                        ));
                    }
                    let Some(capture) = rule.captures.iter().find(|c| c.id == *capture) else {
                        return Err(ItemSourceError::Policy("unknown prerequisite capture"));
                    };
                    if !matches!(&capture.codec, ItemCaptureCodec::Value(codec) if matches!(codec.codec,
                        ValueCodecKind::Integer { .. } | ValueCodecKind::Quantity { .. }))
                    {
                        return Err(ItemSourceError::Policy(
                            "prerequisite capture needs a numeric value codec",
                        ));
                    }
                    // The checked line policy already requires each declared capture
                    // to occur exactly once in its pattern and rejects ambiguous shapes.
                }
            }
        }
        compiled.insert(
            member.rule.clone(),
            PreparedCondition {
                rule_index,
                member_index,
            },
        );
    }
    Ok(compiled)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SourceMemberProof {
    Unproved,
    Single,
    FailedConditions,
}

pub(super) struct SourceMemberContext<'a> {
    pub raw: &'a str,
    pub semantic: &'a str,
    /// Set only for exactly one base recognized in its structural source position.
    pub template: Option<&'a ItemTemplateDefId>,
    pub scaling_syntax_safe: bool,
    pub no_modifier_tags: bool,
    /// Absence under the proven prefix of a fresh source item, not owned defaults.
    pub initial_catalyst_absent: bool,
}

fn decimal_token(token: &str, sign: ItemSourceCaptureSign, min: f64, max: f64) -> bool {
    let body = match sign {
        ItemSourceCaptureSign::Unsigned => Some(token),
        ItemSourceCaptureSign::Plus => token.strip_prefix('+'),
        ItemSourceCaptureSign::Minus => token.strip_prefix('-'),
    };
    let Some(body) = body else { return false };
    if !body.as_bytes().first().is_some_and(u8::is_ascii_digit) {
        return false;
    }
    let mut dot = false;
    if !body
        .bytes()
        .all(|byte| byte.is_ascii_digit() || byte == b'.' && !std::mem::replace(&mut dot, true))
    {
        return false;
    }
    token
        .parse::<f64>()
        .is_ok_and(|value| value.is_finite() && (min..=max).contains(&value))
}

impl ItemSourceLayoutPolicy {
    pub(super) fn proves_single_modifier(
        &self,
        lines: &OwnedItemLinePolicy,
        matched: (Option<&OwnedDefinitionKey>, bool),
        context: SourceMemberContext<'_>,
        work: &mut usize,
        output: &mut usize,
    ) -> Result<SourceMemberProof> {
        let (Some(rule), valid) = matched else {
            return Ok(SourceMemberProof::Unproved);
        };
        if self.roles.get(rule) == Some(&ItemRuleSourceRole::SingleModifier) {
            return Ok(if valid {
                SourceMemberProof::Single
            } else {
                SourceMemberProof::Unproved
            });
        }
        if self.conditions.is_empty() {
            return Ok(SourceMemberProof::Unproved);
        }
        charge(
            work,
            rule.as_str()
                .len()
                .saturating_add(1)
                .saturating_mul(self.conditions.len().saturating_add(1)),
            "work",
        )?;
        let Some(prepared) = self.conditions.get(rule) else {
            return Ok(SourceMemberProof::Unproved);
        };
        if !valid {
            return Ok(SourceMemberProof::FailedConditions);
        }
        let member = &self.input.dialect.single_modifier_conditions()[prepared.member_index];
        charge(work, member.all.len(), "work")?;
        let mut captures = None;
        for condition in &member.all {
            let satisfied = match condition {
                ItemSourceCondition::NoSourceTags => {
                    charge(work, context.raw.len(), "work")?;
                    !context.raw.contains(['{', '}'])
                }
                ItemSourceCondition::NoSourceScalingTags => {
                    context.scaling_syntax_safe && context.no_modifier_tags
                }
                ItemSourceCondition::InitialScalingIsOne => {
                    context.scaling_syntax_safe
                        && (context.no_modifier_tags || context.initial_catalyst_absent)
                }
                ItemSourceCondition::NoGeneratedBuffMembers => {
                    if let Some(template) = context.template {
                        charge(
                            work,
                            template
                                .key()
                                .as_str()
                                .len()
                                .saturating_add(1)
                                .saturating_mul(self.prefixes.len().saturating_add(1)),
                            "work",
                        )?;
                        self.prefixes.get(template)
                            == Some(&ItemLoadIndexPrefix::NoGeneratedBuffMembers)
                    } else {
                        false
                    }
                }
                ItemSourceCondition::UnsignedIntegerCapture { capture, .. }
                | ItemSourceCondition::DecimalCapture { capture, .. } => {
                    if captures.is_none() {
                        captures = lines.source_rule_captures(
                            prepared.rule_index,
                            context.semantic,
                            work,
                            output,
                        )?;
                    }
                    let Some(captures) = &captures else {
                        return Ok(SourceMemberProof::FailedConditions);
                    };
                    charge(
                        work,
                        capture
                            .as_str()
                            .len()
                            .saturating_add(1)
                            .saturating_mul(captures.len().saturating_add(1)),
                        "work",
                    )?;
                    let Some(token) = captures.get(capture) else {
                        return Ok(SourceMemberProof::FailedConditions);
                    };
                    charge(
                        work,
                        token
                            .len()
                            .saturating_mul(
                                if matches!(condition, ItemSourceCondition::DecimalCapture { .. }) {
                                    3
                                } else {
                                    2
                                },
                            )
                            .saturating_add(1),
                        "work",
                    )?;
                    match condition {
                        ItemSourceCondition::UnsignedIntegerCapture { min, max, .. } => {
                            !token.is_empty()
                                && token.bytes().all(|b| b.is_ascii_digit())
                                && token
                                    .parse::<u64>()
                                    .is_ok_and(|value| (*min..=*max).contains(&value))
                        }
                        ItemSourceCondition::DecimalCapture { sign, min, max, .. } => {
                            decimal_token(token, *sign, *min, *max)
                        }
                        _ => unreachable!("numeric condition"),
                    }
                }
            };
            if !satisfied {
                return Ok(SourceMemberProof::FailedConditions);
            }
        }
        Ok(SourceMemberProof::Single)
    }
}
