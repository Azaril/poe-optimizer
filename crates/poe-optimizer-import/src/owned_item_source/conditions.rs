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
            if let ItemSourceCondition::UnsignedIntegerCapture { capture, .. } = condition {
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
    let mut compiled = BTreeMap::new();
    for (member_index, member) in members.iter().enumerate() {
        charge(
            work,
            member.rule.as_str().len().saturating_add(1).saturating_mul(
                known
                    .len()
                    .saturating_add(roles.len())
                    .saturating_add(compiled.len())
                    .saturating_add(3),
            ),
            "schema work",
        )?;
        if member.all.is_empty()
            || roles.get(&member.rule) != Some(&ItemRuleSourceRole::Unresolved)
            || compiled.contains_key(&member.rule)
        {
            return Err(ItemSourceError::Policy(
                "conditional member needs a unique unresolved rule and nonempty prerequisites",
            ));
        }
        let (rule_index, rule) = known
            .get(&member.rule)
            .ok_or(ItemSourceError::Policy("unknown conditional member rule"))?;
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
        let mut captures = BTreeSet::new();
        for condition in &member.all {
            match condition {
                ItemSourceCondition::NoSourceTags => {
                    if std::mem::replace(&mut seen_tags, true) {
                        return Err(ItemSourceError::Policy("duplicate source-tag prerequisite"));
                    }
                }
                ItemSourceCondition::NoGeneratedBuffMembers => {
                    if std::mem::replace(&mut seen_prefix, true) {
                        return Err(ItemSourceError::Policy(
                            "duplicate generated-member prerequisite",
                        ));
                    }
                }
                ItemSourceCondition::UnsignedIntegerCapture { capture, min, max } => {
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
                    if min > max || !captures.insert(capture) {
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
                rule_index: *rule_index,
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
                ItemSourceCondition::UnsignedIntegerCapture { capture, min, max } => {
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
                        token.len().saturating_mul(2).saturating_add(1),
                        "work",
                    )?;
                    !token.is_empty()
                        && token.bytes().all(|b| b.is_ascii_digit())
                        && token
                            .parse::<u64>()
                            .is_ok_and(|value| (*min..=*max).contains(&value))
                }
            };
            if !satisfied {
                return Ok(SourceMemberProof::FailedConditions);
            }
        }
        Ok(SourceMemberProof::Single)
    }
}
