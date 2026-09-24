use super::schema::{value_fits, value_type};
use super::*;

enum Match<'a> {
    No,
    Unique(BTreeMap<OwnedDefinitionKey, &'a str>),
    Ambiguous,
}
fn match_rule<'a>(rule: &ItemLineRule, text: &'a str, work: &mut usize) -> Result<Match<'a>> {
    let mut remaining = text;
    let mut captures = BTreeMap::new();
    for (i, part) in rule.pattern.iter().enumerate() {
        charge(work, 1, "work")?;
        match part {
            ItemPatternPart::Literal(literal) => {
                charge(
                    work,
                    literal.len().min(remaining.len()).saturating_add(1),
                    "work",
                )?;
                let Some(rest) = remaining.strip_prefix(literal) else {
                    return Ok(Match::No);
                };
                remaining = rest;
            }
            ItemPatternPart::NumericCapture {
                capture,
                syntax,
                sign,
            } => {
                charge(work, remaining.len().saturating_add(1), "work")?;
                if matches!(
                    (sign, remaining.as_bytes().first()),
                    (ItemNumericSign::Forbidden, Some(b'+' | b'-'))
                        | (ItemNumericSign::OptionalMinus, Some(b'+'))
                ) {
                    return Ok(Match::No);
                }
                let Some(length) = numeric_prefix_length(remaining, *syntax) else {
                    return Ok(Match::No);
                };
                captures.insert(capture.clone(), &remaining[..length]);
                remaining = &remaining[length..];
            }
            ItemPatternPart::Capture(id) => {
                let length = if let Some(ItemPatternPart::Literal(next)) = rule.pattern.get(i + 1) {
                    charge(work, remaining.len().saturating_add(next.len()), "work")?;
                    let mut positions = remaining.match_indices(next);
                    let Some((first, _)) = positions.next() else {
                        return Ok(Match::No);
                    };
                    // Never guess a capture boundary. An explicit literal delimiter
                    // occurring inside a token needs a more precise injected pattern.
                    if positions.next().is_some() {
                        return Ok(Match::Ambiguous);
                    }
                    first
                } else {
                    remaining.len()
                };
                captures.insert(id.clone(), &remaining[..length]);
                remaining = &remaining[length..];
            }
        }
    }
    if remaining.is_empty() {
        Ok(Match::Unique(captures))
    } else {
        Ok(Match::No)
    }
}
fn unresolved(reason: ItemLinePending, candidates: Vec<OwnedDefinitionKey>) -> ItemLineOutcome {
    ItemLineOutcome::Pending { reason, candidates }
}
impl OwnedItemLinePolicy {
    /// Reuse the checked recipe matcher without decoding or normalizing away spelling.
    /// The source policy binds this rule index to this exact immutable line policy.
    pub(crate) fn source_rule_captures<'a>(
        &self,
        rule_index: usize,
        text: &'a str,
        work: &mut usize,
        output: &mut usize,
    ) -> Result<Option<BTreeMap<OwnedDefinitionKey, &'a str>>> {
        let rule = self
            .input
            .rules
            .get(rule_index)
            .ok_or(ItemLineError::Binding)?;
        if text.len() > self.limits.max_line_bytes {
            return Err(ItemLineError::Limit("line bytes"));
        }
        charge(output, rule.captures.len(), "output declarations")?;
        for capture in &rule.captures {
            charge(work, capture.id.as_str().len().saturating_add(1), "work")?;
        }
        let text = match self.input.whitespace {
            WhitespacePolicy::Exact => text,
            WhitespacePolicy::TrimAscii => {
                charge(work, text.len(), "work")?;
                text.trim_ascii()
            }
        };
        match match_rule(rule, text, work)? {
            Match::Unique(captures) => Ok(Some(captures)),
            Match::No | Match::Ambiguous => Ok(None),
        }
    }
    pub fn convert_line<'a>(
        &self,
        index: usize,
        text: &'a str,
        range_fraction: Option<f64>,
    ) -> Result<ItemLineEvidence<'a>> {
        let mut work = self.limits.max_work;
        let mut output = self.limits.max_output_declarations;
        let mut bytes = self.limits.max_source_bytes;
        self.line(
            ItemLineInput {
                index,
                text,
                range_fraction,
                properties: None,
            },
            &mut work,
            &mut output,
            &mut bytes,
        )
    }
    fn line<'a>(
        &self,
        input: ItemLineInput<'a>,
        work: &mut usize,
        output: &mut usize,
        bytes: &mut usize,
    ) -> Result<ItemLineEvidence<'a>> {
        // Charge supplied facts even for an unknown/malformed line, before any
        // matching, decoding or output cloning. Borrowed maps are never copied.
        charge(bytes, input.text.len(), "source bytes")?;
        if let Some(properties) = input.properties {
            charge(output, properties.len(), "output declarations")?;
            charge(work, properties.len(), "work")?;
            for property in properties.keys() {
                charge(work, property.as_str().len(), "work")?;
                charge(
                    bytes,
                    property.as_str().len().saturating_add(1),
                    "source bytes",
                )?;
            }
        }
        if input.index == 0 {
            return Err(ItemLineError::LineOrder);
        }
        if input.text.len() > self.limits.max_line_bytes {
            return Err(ItemLineError::Limit("line bytes"));
        }
        let evidence = |outcome| ItemLineEvidence {
            index: input.index,
            text: input.text,
            outcome,
        };
        if input.text.contains(['\n', '\r']) {
            return Ok(evidence(unresolved(
                ItemLinePending::UnsupportedLineLayout,
                vec![],
            )));
        }
        let text = match self.input.whitespace {
            WhitespacePolicy::Exact => input.text,
            WhitespacePolicy::TrimAscii => input.text.trim_ascii(),
        };
        let mut matches = vec![];
        for (index, rule) in self.input.rules.iter().enumerate() {
            let matched = match_rule(rule, text, work)?;
            if !matches!(matched, Match::No) {
                matches.push((index, matched));
            }
        }
        if matches.is_empty() {
            return Ok(evidence(unresolved(ItemLinePending::UnknownLine, vec![])));
        }
        charge(output, matches.len(), "output declarations")?;
        let candidates = matches
            .iter()
            .map(|(i, _)| self.input.rules[*i].id.clone())
            .collect();
        if matches.len() != 1 {
            return Ok(evidence(unresolved(
                ItemLinePending::AmbiguousRules,
                candidates,
            )));
        }
        let (index, matched) = matches.pop().expect("one match");
        let Match::Unique(captures) = matched else {
            return Ok(evidence(unresolved(
                ItemLinePending::AmbiguousCapture,
                candidates,
            )));
        };
        let rule = &self.input.rules[index];
        let bound = &self.rules[index];
        let mut closures = bound.modifier_roll_closures.iter();
        for emission in &rule.emissions {
            charge(work, 1, "work")?;
            if let ItemEmission::Modifier { rolls, .. } = emission {
                if let Some(Some(closure)) = closures.next() {
                    charge_roll_closure(closure, work, output)?;
                }
                charge(work, rolls.len(), "work")?;
                for roll in rolls {
                    charge_numeric_projection(&roll.value, work)?;
                    if let ItemLineValue::NumericLexicalProperty { capture, .. } = &roll.value {
                        // Each projection has a bounded lookup and lexical scan.
                        // Charge before resolving, even if another input is pending.
                        let lookup = captures
                            .len()
                            .checked_mul(capture.as_str().len().saturating_add(1))
                            .and_then(|v| v.checked_mul(2))
                            .ok_or(ItemLineError::Limit("work"))?;
                        charge(work, lookup, "work")?;
                        charge(work, captures[capture].len().saturating_add(1), "work")?;
                    }
                    if let ItemLineValue::Property { property } = &roll.value {
                        // A conservative bound covers every key comparison in
                        // the borrowed BTreeMap lookup, including repeated uses.
                        let comparisons = input.properties.map_or(1, |p| p.len().max(1));
                        let cost = comparisons
                            .checked_mul(property.as_str().len().saturating_add(1))
                            .ok_or(ItemLineError::Limit("work"))?;
                        charge(work, cost, "work")?;
                    }
                }
            } else if let ItemEmission::ItemLevel { value }
            | ItemEmission::ItemParameter { value, .. }
            | ItemEmission::TemplateParameter { value, .. }
            | ItemEmission::Quality { amount: value, .. } = emission
            {
                charge_numeric_projection(value, work)?;
            }
        }
        let mut values = BTreeMap::new();
        let mut negative_zeros = BTreeSet::new();
        // Decode present values before reporting schema uncertainty. A malformed
        // matched value never falls through to a second rule or a default tier.
        for (id, codec) in &bound.codecs {
            let raw = captures[id];
            charge(work, raw.len().saturating_add(1), "work")?;
            match codec.decode(raw) {
                Ok(value) => {
                    if self.input.schema_version >= OWNED_ITEM_LINE_POLICY_V4
                        && matches!(&value, ParameterValue::Quantity(q) if q.value() == 0.0)
                    {
                        let ValueCodecKind::Quantity { scale, .. } = &codec.input().codec else {
                            unreachable!("quantity decoded by quantity codec")
                        };
                        // Preserve the decoder's temporary zero sign once. The
                        // lexical token and scale are already validated; scanning
                        // whitespace must not repeat for every consuming recipe.
                        charge(work, raw.len().saturating_add(1), "work")?;
                        if raw.trim_ascii().starts_with('-') ^ (scale.numerator.get() < 0) {
                            negative_zeros.insert(id.clone());
                        }
                    }
                    values.insert(id.clone(), value);
                }
                Err(ValueDecodeError::SourceTooLarge { .. }) => {
                    return Err(ItemLineError::Limit("capture source bytes"));
                }
                Err(e) => {
                    return Ok(evidence(unresolved(
                        ItemLinePending::MalformedCapture {
                            capture: id.clone(),
                            reason: e.to_string(),
                        },
                        candidates,
                    )));
                }
            }
        }
        if let Some(pending) = &bound.pending {
            return Ok(evidence(unresolved(pending.clone(), candidates)));
        }
        for constraint in bound.constraints.iter().flatten() {
            charge(work, 1, "work")?;
            if let ValueSchema::Option { allowed } = constraint {
                charge(work, allowed.members.len(), "work")?;
            }
        }
        let mut constraints = bound.constraints.iter();
        let mut modifier_roll_closures = bound.modifier_roll_closures.iter();
        let mut converted = vec![];
        for emission in &rule.emissions {
            charge(output, 1, "output declarations")?;
            charge(work, 1, "work")?;
            if let ItemEmission::Modifier { rolls, .. } = emission {
                charge(output, rolls.len(), "output declarations")?;
                charge(work, rolls.len(), "work")?;
            }
            let mut get =
                |value: &ItemLineValue, slot: Option<&DeclaredSlot<ParameterSlotDefId>>| {
                    let value = resolve(
                        value,
                        &values,
                        &negative_zeros,
                        &captures,
                        input.range_fraction,
                        input.properties,
                    )?;
                    if let Some(Some(schema)) = constraints.next()
                        && !value_fits(&value, schema)
                    {
                        return Err(ItemLinePending::ValueOutsideSchema {
                            slot: slot.cloned().map(Box::new),
                        });
                    }
                    Ok(value)
                };
            let result = (|| -> std::result::Result<ConvertedItemEmission, ItemLinePending> {
                Ok(match emission {
                    ItemEmission::Metadata { role } => {
                        ConvertedItemEmission::Metadata { role: role.clone() }
                    }
                    ItemEmission::Template { definition } => ConvertedItemEmission::Template {
                        definition: definition.clone(),
                    },
                    ItemEmission::ItemLevel { value } => {
                        let ParameterValue::Integer(value) = get(value, None)? else {
                            unreachable!("validated level kind")
                        };
                        ConvertedItemEmission::ItemLevel { value }
                    }
                    ItemEmission::Quality { kind, amount } => {
                        let ParameterValue::Quantity(amount) = get(amount, None)? else {
                            unreachable!("validated quality kind")
                        };
                        ConvertedItemEmission::Quality {
                            value: QualitySelection {
                                kind: kind.clone(),
                                amount,
                            },
                        }
                    }
                    ItemEmission::ItemParameter { slot, value } => {
                        ConvertedItemEmission::ItemParameter {
                            assignment: ParameterAssignment {
                                slot: slot.clone(),
                                value: get(value, Some(slot))?,
                            },
                        }
                    }
                    ItemEmission::TemplateParameter { value, .. } => {
                        ConvertedItemEmission::TemplateParameter {
                            value: get(value, None)?,
                        }
                    }
                    ItemEmission::Modifier { definition, rolls } => {
                        let mut result = vec![];
                        for r in rolls {
                            result.push(ParameterAssignment {
                                slot: r.slot.clone(),
                                value: get(&r.value, Some(&r.slot))?,
                            });
                        }
                        ConvertedItemEmission::Modifier {
                            definition: definition.clone(),
                            rolls: result,
                            rolls_closure: modifier_roll_closures
                                .next()
                                .and_then(Option::as_ref)
                                .expect("known modifier schema")
                                .clone(),
                        }
                    }
                })
            })();
            match result {
                Ok(v) => converted.push(v),
                Err(reason) => return Ok(evidence(unresolved(reason, candidates))),
            }
        }
        Ok(evidence(ItemLineOutcome::Known {
            rule: rule.id.clone(),
            emissions: converted,
        }))
    }
    /// Physical LF/CRLF lines are preserved without their delimiters. No raw
    /// metadata tags, ModRange overlays or variant selection is interpreted.
    pub fn convert_text<'a>(&self, text: &'a str) -> Result<ItemTextConversion<'a>> {
        if text.len() > self.limits.max_source_bytes {
            return Err(ItemLineError::Limit("source bytes"));
        }
        self.convert_lines(text.split_inclusive('\n').enumerate().map(|(index, line)| {
            let text = line.strip_suffix('\n').unwrap_or(line);
            let text = text.strip_suffix('\r').unwrap_or(text);
            ItemLineInput {
                index: index + 1,
                text,
                range_fraction: None,
                properties: None,
            }
        }))
    }
    pub fn convert_lines<'a>(
        &self,
        lines: impl IntoIterator<Item = ItemLineInput<'a>>,
    ) -> Result<ItemTextConversion<'a>> {
        let mut work = self.limits.max_work;
        let mut output = self.limits.max_output_declarations;
        let mut count = self.limits.max_lines;
        let mut bytes = self.limits.max_source_bytes;
        let mut result = vec![];
        let mut previous = 0;
        for input in lines {
            charge(&mut count, 1, "lines")?;
            if input.index <= previous {
                return Err(ItemLineError::LineOrder);
            }
            previous = input.index;
            result.push(self.line(input, &mut work, &mut output, &mut bytes)?);
        }
        self.aggregate(result, None, &mut work, &mut output)
    }
    /// Internal adapter probe: same grammar and capture validation as conversion,
    /// with the caller's shared work/output budget and no invented range fraction.
    pub(crate) fn probe_source_line<'a>(
        &self,
        index: usize,
        text: &'a str,
        work: &mut usize,
        output: &mut usize,
    ) -> Result<ItemLineEvidence<'a>> {
        let mut bytes = self.limits.max_source_bytes;
        self.line(
            ItemLineInput {
                index,
                text,
                range_fraction: None,
                properties: None,
            },
            work,
            output,
            &mut bytes,
        )
    }
    /// Source blockers retain positively matched candidates for aggregate conflict
    /// handling. The adapter cannot erase a pending competing parameter/header.
    pub(crate) fn convert_source_lines<'a>(
        &self,
        lines: impl IntoIterator<Item = (ItemLineInput<'a>, Option<SourceLinePending<'a>>)>,
        defaults: Option<&ItemInputDefaults>,
        work: &mut usize,
        output: &mut usize,
    ) -> Result<ItemTextConversion<'a>> {
        let mut count = self.limits.max_lines;
        let mut bytes = self.limits.max_source_bytes;
        let mut previous = 0;
        let mut result = Vec::new();
        for (input, pending) in lines {
            charge(&mut count, 1, "lines")?;
            if input.index <= previous {
                return Err(ItemLineError::LineOrder);
            }
            previous = input.index;
            let mut line = self.line(input, work, output, &mut bytes)?;
            if let Some(pending) = pending {
                charge(output, pending.candidates.len(), "output declarations")?;
                line.outcome = unresolved(pending.reason, pending.candidates.to_vec());
            }
            result.push(line);
        }
        self.aggregate(result, defaults, work, output)
    }
    /// Resolve only the selected template. Neither standalone evidence nor a
    /// competing candidate allocates copies of the contextual binding fan-out.
    fn resolve_template_parameters(
        &self,
        result: &mut ItemTextConversion<'_>,
        occurrences: &mut BTreeMap<DeclaredSlot<ParameterSlotDefId>, Vec<usize>>,
        work: &mut usize,
        output: &mut usize,
    ) -> Result<bool> {
        let template = match &result.template {
            ItemField::Known { value, .. } => Some(value),
            _ => None,
        };
        let mut block_defaults = false;
        let mut appended = false;
        for line in &result.lines {
            charge(work, 1, "work")?;
            match &line.outcome {
                ItemLineOutcome::Known { rule, emissions } => {
                    for (emission, value) in emissions.iter().enumerate() {
                        charge(work, 1, "work")?;
                        let ConvertedItemEmission::TemplateParameter { value } = value else {
                            continue;
                        };
                        charge(
                            work,
                            (rule.as_str().len() + 1).saturating_mul(self.rule_indices.len() + 1),
                            "work",
                        )?;
                        let bound = &self.rules[self.rule_indices[rule]];
                        let targets = &bound.template_parameters[&emission];
                        let Some(target) = selected_template_parameter(targets, template, work)?
                        else {
                            charge(output, 1, "output declarations")?;
                            result.issues.push(ItemTextIssue {
                                problem: if template.is_some() {
                                    ItemTextProblem::TemplateParameterUnavailable
                                } else {
                                    ItemTextProblem::TemplateUnavailable
                                },
                                lines: vec![line.index],
                            });
                            block_defaults = true;
                            continue;
                        };
                        record_parameter_occurrence(
                            occurrences,
                            &target.slot,
                            line.index,
                            false,
                            work,
                            output,
                        )?;
                        let Some(schema) =
                            target.schema.as_ref().filter(|_| target.pending.is_none())
                        else {
                            charge(output, 1, "output declarations")?;
                            result.issues.push(ItemTextIssue {
                                problem: ItemTextProblem::TemplateParameterUnavailable,
                                lines: vec![line.index],
                            });
                            continue;
                        };
                        charge(work, 1, "work")?;
                        if let ValueSchema::Option { allowed } = schema {
                            charge(work, allowed.members.len(), "work")?;
                        }
                        if !value_fits(value, schema) {
                            charge(output, 1, "output declarations")?;
                            result.issues.push(ItemTextIssue {
                                problem: ItemTextProblem::ParameterOutsideSchema,
                                lines: vec![line.index],
                            });
                            continue;
                        }
                        charge(output, 1, "output declarations")?;
                        result.parameters.push(LocatedItemParameter {
                            line: line.index,
                            emission,
                            assignment: ParameterAssignment {
                                slot: target.slot.clone(),
                                value: value.clone(),
                            },
                        });
                        appended = true;
                    }
                }
                ItemLineOutcome::Pending { candidates, .. } => {
                    for rule in candidates {
                        charge(
                            work,
                            (rule.as_str().len() + 1).saturating_mul(self.rule_indices.len() + 1),
                            "work",
                        )?;
                        let Some(index) = self.rule_indices.get(rule) else {
                            continue;
                        };
                        for targets in self.rules[*index].template_parameters.values() {
                            charge(work, 1, "work")?;
                            if let Some(target) =
                                selected_template_parameter(targets, template, work)?
                            {
                                // An unresolved sibling cannot allow fallback or
                                // leave an otherwise matching explicit alias valid.
                                record_parameter_occurrence(
                                    occurrences,
                                    &target.slot,
                                    line.index,
                                    true,
                                    work,
                                    output,
                                )?;
                            } else {
                                charge(output, 1, "output declarations")?;
                                result.issues.push(ItemTextIssue {
                                    problem: if template.is_some() {
                                        ItemTextProblem::TemplateParameterUnavailable
                                    } else {
                                        ItemTextProblem::TemplateUnavailable
                                    },
                                    lines: vec![line.index],
                                });
                                block_defaults = true;
                            }
                        }
                    }
                }
            }
        }
        if appended {
            let count = result.parameters.len();
            let levels = count.checked_ilog2().unwrap_or(0) as usize + 1;
            charge(work, count.saturating_mul(levels), "work")?;
            result
                .parameters
                .sort_by_key(|parameter| (parameter.line, parameter.emission));
        }
        Ok(block_defaults)
    }
    fn aggregate<'a>(
        &self,
        lines: Vec<ItemLineEvidence<'a>>,
        defaults: Option<&ItemInputDefaults>,
        work: &mut usize,
        output: &mut usize,
    ) -> Result<ItemTextConversion<'a>> {
        let mut result = ItemTextConversion {
            lines,
            template: ItemField::Absent,
            item_level: ItemField::Absent,
            quality: ItemField::Absent,
            modifiers: vec![],
            parameters: vec![],
            issues: vec![],
            defaults: ItemDefaultedInputs::default(),
        };
        let mut occurrences: BTreeMap<DeclaredSlot<ParameterSlotDefId>, Vec<usize>> =
            BTreeMap::new();
        for line in &result.lines {
            if let ItemLineOutcome::Known { emissions, .. } = &line.outcome {
                for (emission, value) in emissions.iter().enumerate() {
                    charge(work, 1, "work")?;
                    match value {
                        ConvertedItemEmission::Template { definition } => set_field(
                            &mut result.template,
                            definition.clone(),
                            line.index,
                            &mut result.issues,
                        ),
                        ConvertedItemEmission::ItemLevel { value } => set_field(
                            &mut result.item_level,
                            *value,
                            line.index,
                            &mut result.issues,
                        ),
                        ConvertedItemEmission::Quality { value } => set_field(
                            &mut result.quality,
                            value.clone(),
                            line.index,
                            &mut result.issues,
                        ),
                        ConvertedItemEmission::ItemParameter { assignment } => {
                            occurrences
                                .entry(assignment.slot.clone())
                                .or_default()
                                .push(line.index);
                            result.parameters.push(LocatedItemParameter {
                                line: line.index,
                                emission,
                                assignment: assignment.clone(),
                            })
                        }
                        ConvertedItemEmission::Modifier {
                            definition,
                            rolls,
                            rolls_closure,
                        } => {
                            charge_roll_closure(rolls_closure, work, output)?;
                            result.modifiers.push(LocatedItemModifier {
                                line: line.index,
                                emission,
                                definition: definition.clone(),
                                rolls: rolls.clone(),
                                rolls_closure: rolls_closure.clone(),
                            });
                        }
                        _ => {}
                    }
                }
            } else if let ItemLineOutcome::Pending { candidates, .. } = &line.outcome {
                // A positively matched but unresolved header cannot be hidden by
                // another successful alias. Unknown lines remain general evidence.
                for id in candidates {
                    charge(work, 1, "work")?;
                    if let Some(index) = self.rule_indices.get(id) {
                        let rule = &self.input.rules[*index];
                        for e in &rule.emissions {
                            charge(work, 1, "work")?;
                            charge(output, 1, "output declarations")?;
                            match e {
                                ItemEmission::ItemParameter { slot, .. } => {
                                    let origins = occurrences.entry(slot.clone()).or_default();
                                    if origins.last() != Some(&line.index) {
                                        origins.push(line.index);
                                    }
                                }

                                ItemEmission::Template { .. } => {
                                    pending_field(&mut result.template, line.index)
                                }
                                ItemEmission::ItemLevel { .. } => {
                                    pending_field(&mut result.item_level, line.index)
                                }
                                ItemEmission::Quality { .. } => {
                                    pending_field(&mut result.quality, line.index)
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        let contextual_defaults_blocked = if self.input.schema_version >= OWNED_ITEM_LINE_POLICY_V5
        {
            self.resolve_template_parameters(&mut result, &mut occurrences, work, output)?
        } else {
            false
        };
        for indices in occurrences.values().filter(|v| v.len() > 1) {
            result.issues.push(ItemTextIssue {
                problem: ItemTextProblem::DuplicateParameter,
                lines: indices.clone(),
            });
        }
        result
            .parameters
            .retain(|p| occurrences[&p.assignment.slot].len() == 1);
        let ItemField::Known {
            value: template, ..
        } = &result.template
        else {
            let mut unresolved: Vec<_> = result
                .modifiers
                .iter()
                .map(|v| v.line)
                .chain(result.parameters.iter().map(|v| v.line))
                .collect();
            if let ItemField::Known { line, .. } = &result.item_level {
                let line = *line;
                unresolved.push(line);
                pending_field(&mut result.item_level, line);
            }
            if let ItemField::Known { line, .. } = &result.quality {
                let line = *line;
                unresolved.push(line);
                pending_field(&mut result.quality, line);
            }
            if !unresolved.is_empty() {
                result.issues.push(ItemTextIssue {
                    problem: ItemTextProblem::TemplateUnavailable,
                    lines: unresolved,
                });
            }
            result.modifiers.clear();
            result.parameters.clear();
            return Ok(result);
        };
        let Some(context) = self.templates.get(template) else {
            unreachable!("known template has validated context")
        };
        let owner = SlotOwnerDefId::ItemTemplate(template.clone());
        if let Some(defaults) = defaults.filter(|d| &d.template == template) {
            charge(work, defaults.values.parameters.len(), "work")?;
            for assignment in &defaults.values.parameters {
                // A positive or unresolved authored occurrence always suppresses fallback.
                let cost =
                    (assignment.slot.slot.key().as_str().len() + template.key().as_str().len() + 1)
                        .saturating_mul(occurrences.len().saturating_add(1));
                charge(work, cost, "work")?;
                if !contextual_defaults_blocked && !occurrences.contains_key(&assignment.slot) {
                    charge(output, 1, "output declarations")?;
                    result.defaults.parameters.push(assignment.clone());
                }
            }
            result.defaults.item_level_absent =
                defaults.values.item_level_absent && matches!(result.item_level, ItemField::Absent);
            result.defaults.quality_absent =
                defaults.values.quality_absent && matches!(result.quality, ItemField::Absent);
        }
        result.parameters.retain(|p| {
            if p.assignment.slot.declaration == owner {
                true
            } else {
                result.issues.push(ItemTextIssue {
                    problem: ItemTextProblem::WrongParameterOwner,
                    lines: vec![p.line],
                });
                false
            }
        });
        let mut kept = vec![];
        for m in result.modifiers.drain(..) {
            charge(
                work,
                context.modifiers.members.len().saturating_add(1),
                "work",
            )?;
            if context.modifiers.members.contains(&m.definition) {
                kept.push(m)
            } else {
                result.issues.push(ItemTextIssue {
                    problem: if context.modifiers.is_complete() {
                        ItemTextProblem::ModifierNotAllowed
                    } else {
                        ItemTextProblem::SchemaPartial
                    },
                    lines: vec![m.line],
                });
            }
        }
        result.modifiers = kept;
        if let ItemField::Known { value, line } = &result.item_level
            && (value < &context.level.minimum || value > &context.level.maximum)
        {
            let line = *line;
            result.issues.push(ItemTextIssue {
                problem: ItemTextProblem::LevelOutsideSchema,
                lines: vec![line],
            });
            pending_field(&mut result.item_level, line);
        }
        if let ItemField::Known { value, line } = &result.quality {
            charge(
                work,
                context
                    .quality
                    .allowed_kinds
                    .members
                    .len()
                    .saturating_add(1),
                "work",
            )?;
            if context.quality.presence == QualityPresence::Forbidden
                || !context.quality.allowed_kinds.members.contains(&value.kind)
            {
                let problem = if context.quality.presence != QualityPresence::Forbidden
                    && !context.quality.allowed_kinds.is_complete()
                {
                    ItemTextProblem::SchemaPartial
                } else {
                    ItemTextProblem::QualityNotAllowed
                };
                let line = *line;
                result.issues.push(ItemTextIssue {
                    problem,
                    lines: vec![line],
                });
                pending_field(&mut result.quality, line);
            }
        }
        if context.quality.presence == QualityPresence::Required
            && matches!(result.quality, ItemField::Absent)
        {
            result.issues.push(ItemTextIssue {
                problem: ItemTextProblem::RequiredQualityMissing,
                lines: vec![],
            });
        }
        let supplied: BTreeSet<_> = result
            .parameters
            .iter()
            .map(|p| &p.assignment.slot)
            .chain(result.defaults.parameters.iter().map(|p| &p.slot))
            .collect();
        charge(
            work,
            context.required_parameters.len() + supplied.len(),
            "work",
        )?;
        if context
            .required_parameters
            .iter()
            .any(|s| !supplied.contains(s))
        {
            result.issues.push(ItemTextIssue {
                problem: ItemTextProblem::RequiredParameterMissing,
                lines: vec![],
            });
        }
        if !context.parameters_complete {
            result.issues.push(ItemTextIssue {
                problem: ItemTextProblem::SchemaPartial,
                lines: vec![],
            });
        }
        Ok(result)
    }
}
fn set_field<T>(field: &mut ItemField<T>, value: T, line: usize, issues: &mut Vec<ItemTextIssue>) {
    match field {
        ItemField::Absent => *field = ItemField::Known { value, line },
        _ => {
            pending_field(field, line);
            if let ItemField::Pending { lines } = field {
                issues.push(ItemTextIssue {
                    problem: ItemTextProblem::DuplicateHeader,
                    lines: if lines[0] == line {
                        vec![line]
                    } else {
                        vec![lines[0], line]
                    },
                });
            }
        }
    }
}
fn pending_field<T>(field: &mut ItemField<T>, line: usize) {
    match field {
        ItemField::Known { line: first, .. } => {
            let first = *first;
            *field = ItemField::Pending {
                lines: if first == line {
                    vec![line]
                } else {
                    vec![first, line]
                },
            };
        }
        ItemField::Pending { lines } => {
            if lines.last() != Some(&line) {
                lines.push(line)
            }
        }
        ItemField::Absent => *field = ItemField::Pending { lines: vec![line] },
    }
}
fn resolve(
    value: &ItemLineValue,
    values: &BTreeMap<OwnedDefinitionKey, ParameterValue>,
    negative_zeros: &BTreeSet<OwnedDefinitionKey>,
    captures: &BTreeMap<OwnedDefinitionKey, &str>,
    fraction: Option<f64>,
    properties: Option<&BTreeMap<OwnedDefinitionKey, bool>>,
) -> std::result::Result<ParameterValue, ItemLinePending> {
    match value {
        ItemLineValue::Literal(v) => Ok(v.clone()),
        ItemLineValue::Capture(id) => Ok(values[id].clone()),
        ItemLineValue::NumericLexicalProperty { capture, property } => {
            // Pattern and numeric codec validation have both succeeded. Spelling
            // is adapter evidence only; outputs contain just the explicit fact.
            Ok(ParameterValue::Boolean(match property {
                ItemNumericLexicalProperty::HasDecimalPoint => captures[capture].contains('.'),
            }))
        }
        ItemLineValue::NumericProjection(projection) => {
            numeric_projection(projection, values, negative_zeros, fraction)
        }
        ItemLineValue::Property { property } => properties
            .and_then(|values| values.get(property))
            .copied()
            .map(ParameterValue::Boolean)
            .ok_or_else(|| ItemLinePending::MissingProperty {
                property: property.clone(),
            }),
        ItemLineValue::InterpolateUnroundedOffset { lower, upper } => {
            // V3 keeps canonical decoded quantities, including normalized zero.
            // V4's separate projection can preserve the temporary lexical sign.
            let (ParameterValue::Quantity(a), ParameterValue::Quantity(b)) =
                (&values[lower], &values[upper])
            else {
                return Err(ItemLinePending::InvalidRange);
            };
            if a.unit() != b.unit() {
                return Err(ItemLinePending::InvalidRange);
            }
            let raw = unrounded_offset(a.value(), b.value(), fraction)?;
            Ok(ParameterValue::Quantity(
                FiniteQuantity::new(raw, a.unit().clone())
                    .map_err(|_| ItemLinePending::InvalidRange)?,
            ))
        }
        ItemLineValue::Interpolate {
            lower,
            upper,
            quantum,
            rounding,
        }
        | ItemLineValue::InterpolateOffset {
            lower,
            upper,
            quantum,
            rounding,
        } => {
            let fraction = fraction.ok_or(ItemLinePending::MissingRangeFraction)?;
            if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
                return Err(ItemLinePending::InvalidRangeFraction);
            }
            let a = &values[lower];
            let b = &values[upper];
            if value_type(a) != value_type(b) || value_type(a) != value_type(quantum) {
                return Err(ItemLinePending::InvalidRange);
            }
            let (a, b, q) = match (a, b, quantum) {
                (
                    ParameterValue::Integer(a),
                    ParameterValue::Integer(b),
                    ParameterValue::Integer(q),
                ) => (a.get() as f64, b.get() as f64, q.get() as f64),
                (
                    ParameterValue::Quantity(a),
                    ParameterValue::Quantity(b),
                    ParameterValue::Quantity(q),
                ) => (a.value(), b.value(), q.value()),
                _ => return Err(ItemLinePending::InvalidRange),
            };
            if a > b {
                return Err(ItemLinePending::InvalidRange);
            }
            // The offset operation preserves each literal intermediate. The
            // existing operation keeps exact endpoints and uses a convex
            // combination across opposite-sign bounds to avoid overflow of b-a.
            let raw = if matches!(value, ItemLineValue::InterpolateOffset { .. }) {
                let difference = b - a;
                let offset = fraction * difference;
                if !difference.is_finite() || !offset.is_finite() {
                    return Err(ItemLinePending::InvalidRange);
                }
                a + offset
            } else if fraction == 0.0 {
                a
            } else if fraction == 1.0 {
                b
            } else if a.signum() == b.signum() {
                a + (b - a) * fraction
            } else {
                a * (1.0 - fraction) + b * fraction
            };
            let scaled = raw / q;
            if !raw.is_finite() || !scaled.is_finite() {
                return Err(ItemLinePending::InvalidRange);
            }
            let rounded = match rounding {
                ItemRangeRounding::Floor => {
                    if scaled == 0.0 && raw < 0.0 {
                        -1.0
                    } else {
                        scaled.floor()
                    }
                }
                ItemRangeRounding::Ceiling => {
                    if scaled == 0.0 && raw > 0.0 {
                        1.0
                    } else {
                        scaled.ceil()
                    }
                }
                ItemRangeRounding::Truncate => scaled.trunc(),
                // Preserve the source import's literal IEEE offset order;
                // mathematically nearest f64::round() is not equivalent.
                ItemRangeRounding::SymmetricHalfOffset => {
                    if scaled >= 0.0 {
                        (scaled + 0.5).floor()
                    } else {
                        (scaled - 0.5).ceil()
                    }
                }
                ItemRangeRounding::NearestTiesPositive => {
                    let floor = scaled.floor();
                    if scaled - floor < 0.5 {
                        floor
                    } else {
                        floor + 1.0
                    }
                }
            } * q;
            if !rounded.is_finite() {
                return Err(ItemLinePending::InvalidRange);
            }
            match quantum {
                ParameterValue::Integer(_) => {
                    if rounded.fract() != 0.0 || rounded.abs() > 9_007_199_254_740_991.0 {
                        return Err(ItemLinePending::InvalidRange);
                    }
                    Ok(ParameterValue::Integer(
                        BoundedInteger::new(rounded as i64)
                            .map_err(|_| ItemLinePending::InvalidRange)?,
                    ))
                }
                ParameterValue::Quantity(q) => Ok(ParameterValue::Quantity(
                    FiniteQuantity::new(rounded, q.unit().clone())
                        .map_err(|_| ItemLinePending::InvalidRange)?,
                )),
                _ => unreachable!("numeric quantum"),
            }
        }
    }
}

// Bound all numeric stages before allocation/formatting. At most 17 significant
// digits, sign, decimal point, exponent marker/sign and three exponent digits
// are formatted; the fixed charge includes source/projection arithmetic.
fn charge_numeric_projection(value: &ItemLineValue, work: &mut usize) -> Result<()> {
    if matches!(value, ItemLineValue::NumericProjection(_)) {
        charge(work, 64, "work")?;
    }
    Ok(())
}

fn unrounded_offset(
    a: f64,
    b: f64,
    fraction: Option<f64>,
) -> std::result::Result<f64, ItemLinePending> {
    let fraction = fraction.ok_or(ItemLinePending::MissingRangeFraction)?;
    if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
        return Err(ItemLinePending::InvalidRangeFraction);
    }
    if a > b {
        return Err(ItemLinePending::InvalidRange);
    }
    // Preserve each ordinary IEEE operation, including at fraction 0/1.
    // A convex form or an endpoint shortcut would change this contract.
    let difference = b - a;
    if !difference.is_finite() {
        return Err(ItemLinePending::InvalidRange);
    }
    let offset = fraction * difference;
    if !offset.is_finite() {
        return Err(ItemLinePending::InvalidRange);
    }
    let raw = a + offset;
    if !raw.is_finite() {
        return Err(ItemLinePending::InvalidRange);
    }
    Ok(raw)
}

fn numeric_projection(
    projection: &ItemNumericProjection,
    values: &BTreeMap<OwnedDefinitionKey, ParameterValue>,
    negative_zeros: &BTreeSet<OwnedDefinitionKey>,
    fraction: Option<f64>,
) -> std::result::Result<ParameterValue, ItemLinePending> {
    let capture = |id: &OwnedDefinitionKey| {
        let ParameterValue::Quantity(quantity) = &values[id] else {
            unreachable!("validated numeric projection quantity")
        };
        let mut value = quantity.value();
        // Core quantities normalize zero. The charged decode step retained
        // its temporary sign, including underflow or a zero scale, once.
        if value == 0.0 && negative_zeros.contains(id) {
            value = -0.0;
        }
        (value, quantity.unit())
    };
    let (mut value, unit) = match &projection.source {
        ItemNumericSource::Capture(id) => capture(id),
        ItemNumericSource::InterpolateUnroundedOffset { lower, upper } => {
            let (a, unit) = capture(lower);
            let (b, other_unit) = capture(upper);
            debug_assert_eq!(unit, other_unit);
            (unrounded_offset(a, b, fraction)?, unit)
        }
    };
    if projection.negate {
        value = -value;
    }
    if let ItemNumericDecimal::SignificantDigits { digits } = projection.decimal {
        // Construction checks 1..=17 before any formatting allocation.
        let precision = usize::from(digits - 1);
        value = format!("{value:.precision$e}")
            .parse::<f64>()
            .map_err(|_| ItemLinePending::InvalidNumericProjection)?;
        if !value.is_finite() {
            return Err(ItemLinePending::InvalidNumericProjection);
        }
    }
    match projection.result {
        ItemNumericResult::NegativeDirection { invert } => {
            Ok(ParameterValue::Boolean(value.is_sign_negative() ^ invert))
        }
        ItemNumericResult::SignedQuantity | ItemNumericResult::Magnitude => {
            if projection.result == ItemNumericResult::Magnitude {
                value = value.abs();
            }
            Ok(ParameterValue::Quantity(
                FiniteQuantity::new(value, unit.clone())
                    .map_err(|_| ItemLinePending::InvalidNumericProjection)?,
            ))
        }
    }
}

fn charge_roll_closure(
    closure: &SchemaClosure,
    work: &mut usize,
    output: &mut usize,
) -> Result<()> {
    if let SchemaClosure::Partial { gaps } = closure {
        charge(work, gaps.len(), "work")?;
        charge(output, gaps.len(), "output declarations")?;
    }
    Ok(())
}

fn selected_template_parameter<'a>(
    targets: &'a BTreeMap<ItemTemplateDefId, BoundTemplateParameter>,
    template: Option<&ItemTemplateDefId>,
    work: &mut usize,
) -> Result<Option<&'a BoundTemplateParameter>> {
    let Some(template) = template else {
        return Ok(None);
    };
    charge(
        work,
        (template.key().as_str().len() + 1).saturating_mul(targets.len() + 1),
        "work",
    )?;
    Ok(targets.get(template))
}

fn record_parameter_occurrence(
    occurrences: &mut BTreeMap<DeclaredSlot<ParameterSlotDefId>, Vec<usize>>,
    slot: &DeclaredSlot<ParameterSlotDefId>,
    line: usize,
    deduplicate_line: bool,
    work: &mut usize,
    output: &mut usize,
) -> Result<()> {
    let key_bytes = slot.slot.key().as_str().len() + slot.declaration.key().as_str().len() + 1;
    charge(
        work,
        key_bytes.saturating_mul(occurrences.len() + 1),
        "work",
    )?;
    charge(output, 1, "output declarations")?;
    let lines = occurrences.entry(slot.clone()).or_default();
    if !deduplicate_line || lines.last() != Some(&line) {
        lines.push(line);
    }
    Ok(())
}
