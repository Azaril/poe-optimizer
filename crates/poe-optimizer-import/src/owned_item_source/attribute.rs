use super::conditions::{SourceMemberContext, SourceMemberProof};
use super::*;
fn problem(problems: &mut Vec<ItemSourceProblem>, p: ItemSourceProblem) {
    if !problems.contains(&p) {
        problems.push(p);
    }
}
fn fraction(text: &str) -> Option<f64> {
    let value = text.parse::<f64>().ok()?;
    (value.is_finite() && (0.0..=1.0).contains(&value)).then_some(value)
}
fn integer(text: &str) -> Option<usize> {
    // Supported exported integer syntax. Do not inherit Lua tonumber's arbitrary coercions.
    (!text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()))
        .then(|| text.parse().ok())
        .flatten()
}
fn field<'a>(
    row: &'a SourceEvidenceRow<'_>,
    name: &str,
) -> (Option<SourceAttributeRef>, Option<&'a str>) {
    let mut found = None;
    for (i, a) in row.attributes().iter().enumerate() {
        if a.origin().namespace.is_none() && a.origin().name == name {
            if found.is_some() {
                return (None, None);
            }
            found = Some((
                SourceAttributeRef {
                    occurrence: row.occurrence().id(),
                    index: i as u32,
                },
                a.decoded().ok(),
            ));
        }
    }
    found.map_or((None, None), |(id, value)| (Some(id), value))
}
struct Tags {
    semantic: String,
    enchant: bool,
    implicit: bool,
    tagged: bool,
    scaling_syntax_safe: bool,
}
fn tags(
    line: &mut ItemAttributedLine,
    bindings: &BTreeMap<String, OwnedDefinitionKey>,
    flags: Option<&BTreeMap<ItemSourceLineFlag, OwnedDefinitionKey>>,
    report: &mut Vec<ItemRangeWrite>,
    tag_left: &mut usize,
    output: &mut usize,
    work: &mut usize,
) -> Result<Tags> {
    let text = line.raw.trim_ascii();
    let leading = line.raw.len() - line.raw.trim_ascii_start().len();
    let mut at = 0;
    let mut semantic = String::new();
    let mut enchant = false;
    let mut implicit = false;
    let mut tagged = false;
    let mut scaling_syntax_safe = true;
    while let Some(relative) = text[at..].find('{') {
        tagged = true;
        let start = at + relative;
        charge(work, text.len() - at, "work")?;
        semantic.push_str(&text[at..start]);
        let Some(end) = text[start + 1..].find('}').map(|v| v + start + 1) else {
            problem(&mut line.blockers, ItemSourceProblem::MalformedTag);
            scaling_syntax_safe = false;
            semantic.push_str(&text[start..]);
            at = text.len();
            break;
        };
        charge(tag_left, 1, "tags")?;
        let tag = &text[start + 1..end];
        if let Some(value) = tag.strip_prefix("range:") {
            charge(output, 1, "output records")?;
            let value = fraction(value);
            let write = report.len();
            report.push(ItemRangeWrite {
                origin: ItemRangeOrigin::Inline {
                    line: line.index,
                    span: (line.decoded_span.start + leading + start)
                        ..(line.decoded_span.start + leading + end + 1),
                },
                source_id: None,
                fraction: value,
                target: ItemRangeTarget::Line(line.index),
            });
            line.range = match value {
                Some(fraction) => ItemRangeDecision::Resolved {
                    fraction,
                    winning_write: write,
                },
                None => ItemRangeDecision::Pending,
            };
            if value.is_none() {
                problem(&mut line.blockers, ItemSourceProblem::InvalidRange);
            }
        } else if tag == "tags" || tag.starts_with("tags:") {
            // The reviewed source grammar extracts ASCII alphabetic/underscore
            // runs. Separators add no tokens; case and explicit aliases stay exact.
            let value = tag.strip_prefix("tags:").unwrap_or("");
            let start_of_value = line.decoded_span.start + leading + start + 6;
            charge(work, value.len(), "work")?;
            let mut at = 0;
            while at < value.len() {
                if !(value.as_bytes()[at].is_ascii_alphabetic() || value.as_bytes()[at] == b'_') {
                    at += 1;
                    continue;
                }
                let begin = at;
                while at < value.len()
                    && (value.as_bytes()[at].is_ascii_alphabetic() || value.as_bytes()[at] == b'_')
                {
                    at += 1;
                }
                let label = &value[begin..at];
                charge(tag_left, 1, "tags")?;
                charge(output, 1, "output records")?;
                comparison_work(work, label.len(), bindings.len())?;
                let property = bindings.get(label);
                if let Some(property) = property {
                    charge(work, property.as_str().len(), "work")?;
                } else {
                    problem(&mut line.blockers, ItemSourceProblem::UnknownProperty);
                }
                line.property_tokens.push(ItemSourcePropertyToken {
                    label: label.into(),
                    decoded_span: start_of_value + begin..start_of_value + at,
                    property: property.cloned(),
                });
            }
        } else if flags.is_some() && matches!(tag, "fractured" | "desecrated") {
            let label = if tag == "fractured" {
                ItemSourceLineFlag::Fractured
            } else {
                ItemSourceLineFlag::Desecrated
            };
            let bindings = flags.expect("checked flag dialect");
            comparison_work(work, tag.len(), bindings.len())?;
            let property = bindings.get(&label);
            if let Some(property) = property {
                charge(work, property.as_str().len(), "work")?;
            }
            let property = property.cloned();
            charge(output, 1, "output records")?;
            if property.is_none() {
                problem(&mut line.blockers, ItemSourceProblem::UnsupportedTag);
            }
            line.flag_tokens.push(ItemSourceFlagToken {
                label,
                decoded_span: (line.decoded_span.start + leading + start)
                    ..(line.decoded_span.start + leading + end + 1),
                property,
            });
        } else {
            match tag {
                "enchant" => {
                    enchant = true;
                    implicit = true;
                }
                "implicit" => implicit = true,
                "rune" => {
                    scaling_syntax_safe = false;
                    problem(&mut line.blockers, ItemSourceProblem::RuneLifecycle);
                }
                _ => {
                    scaling_syntax_safe = false;
                    problem(&mut line.blockers, ItemSourceProblem::UnsupportedTag);
                }
            }
        }
        at = end + 1;
    }
    semantic.push_str(&text[at..]);
    if semantic.contains('}') {
        scaling_syntax_safe = false;
        problem(&mut line.blockers, ItemSourceProblem::MalformedTag);
    }
    Ok(Tags {
        semantic,
        enchant,
        implicit,
        tagged,
        scaling_syntax_safe,
    })
}
fn matched(evidence: &ItemLineEvidence<'_>) -> (Option<OwnedDefinitionKey>, bool) {
    match &evidence.outcome {
        ItemLineOutcome::Known { rule, .. } => (Some(rule.clone()), true),
        ItemLineOutcome::Pending { reason, candidates } if candidates.len() == 1 => (
            Some(candidates[0].clone()),
            !matches!(
                reason,
                ItemLinePending::UnknownLine
                    | ItemLinePending::AmbiguousRules
                    | ItemLinePending::AmbiguousCapture
                    | ItemLinePending::MalformedCapture { .. }
                    // A reviewed source role cannot use an owned input bound as
                    // a proof when the actual token falls outside that bound.
                    // Source formatting may then fail or combine the next line.
                    | ItemLinePending::ValueOutsideSchema { .. }
                    | ItemLinePending::UnsupportedLineLayout
            ),
        ),
        _ => (None, false),
    }
}
fn collect_candidates(
    evidence: &ItemLineEvidence<'_>,
    into: &mut Vec<OwnedDefinitionKey>,
    output: &mut usize,
    work: &mut usize,
) -> Result<()> {
    let candidates: &[OwnedDefinitionKey] = match &evidence.outcome {
        ItemLineOutcome::Known { rule, .. } => std::slice::from_ref(rule),
        ItemLineOutcome::Pending { candidates, .. } => candidates,
    };
    for id in candidates {
        charge(
            work,
            into.len()
                .saturating_add(1)
                .saturating_mul(id.as_str().len().saturating_add(1)),
            "work",
        )?;
        if !into.contains(id) {
            charge(output, 1, "output records")?;
            into.push(id.clone());
        }
    }
    Ok(())
}
// A conservative bound independent of the standard library tree's fanout and
// comparison strategy. Charge before variable-length lookup/insert comparisons.
fn comparison_work(work: &mut usize, key_bytes: usize, entries: usize) -> Result<()> {
    charge(
        work,
        key_bytes
            .saturating_add(1)
            .saturating_mul(entries.saturating_add(1)),
        "work",
    )
}
impl ItemSourceLayoutPolicy {
    fn bind_properties(
        &self,
        line: &mut ItemAttributedLine,
        rule: &OwnedDefinitionKey,
        work: &mut usize,
        output: &mut usize,
    ) -> Result<()> {
        let required = self.rule_properties.get(rule);
        charge(
            work,
            line.property_tokens
                .len()
                .saturating_add(line.flag_tokens.len()),
            "work",
        )?;
        for property in line
            .property_tokens
            .iter()
            .map(|token| &token.property)
            .chain(line.flag_tokens.iter().map(|token| &token.property))
            .flatten()
        {
            comparison_work(
                work,
                property.as_str().len(),
                required.map_or(0, BTreeSet::len),
            )?;
            if !required.is_some_and(|keys| keys.contains(property)) {
                problem(&mut line.blockers, ItemSourceProblem::UnconsumedProperty);
            }
        }
        // Recognition without a typed output cannot silently discard a label.
        // No false values are synthesized for a malformed/unsupported member.
        if (!line.blockers.is_empty() && self.input.dialect.tracks_flags())
            || line
                .blockers
                .iter()
                .any(|p| !matches!(p, ItemSourceProblem::InvalidRange))
        {
            return Ok(());
        }
        let Some(required) = required else {
            return Ok(());
        };
        charge(output, required.len(), "output records")?;
        charge(work, required.len(), "work")?;
        let mut present = BTreeSet::new();
        for property in line
            .property_tokens
            .iter()
            .filter_map(|token| token.property.as_ref())
            .chain(
                line.flag_tokens
                    .iter()
                    .filter_map(|token| token.property.as_ref()),
            )
        {
            comparison_work(work, property.as_str().len(), present.len())?;
            present.insert(property);
        }
        for property in required {
            comparison_work(work, property.as_str().len(), present.len())?;
            let value = present.contains(property);
            comparison_work(work, property.as_str().len(), line.properties.len())?;
            line.properties.insert(property.clone(), value);
        }
        Ok(())
    }
    pub fn attribute(
        &self,
        evidence: &SourceProjectEvidence<'_>,
        item: SourceOccurrenceId,
        lines: &OwnedItemLinePolicy,
    ) -> Result<ItemRangeAttribution> {
        if self.input.item_lines != *lines.identity() {
            return Err(ItemSourceError::Binding);
        }
        let row = evidence.row(item)?;
        let root = evidence.rows().first().ok_or(ItemSourceError::SourceKind)?;
        if root.occurrence().name() != "PathOfBuilding2"
            || root.occurrence().has_namespace_context()
            || row.occurrence().name() != "Item"
            || row.occurrence().has_namespace_context()
            || !matches!(
                row.authored_instance(),
                Some(AuthoredInstanceId::ItemRecord(_))
            )
        {
            return Err(ItemSourceError::SourceKind);
        }
        let identity = evidence.identity();
        let mut report = ItemAttributionReport {
            source: ItemSourceBinding {
                source_sha256: identity.source_sha256.into(),
                source_bytes: identity.source_bytes,
                source_schema: identity.instance_import_schema,
                lineage: identity.lineage,
                revision: identity.revision,
                allocator: identity.allocator,
            },
            item,
            content_entry: None,
            policy: self.identity,
            item_lines: *lines.identity(),
            layout: ItemLayoutStatus::Proven,
            lines: vec![],
            writes: vec![],
            default_scope: ItemSourceDefaultScope::Unconfigured,
        };
        let mut work = self.limits.max_work.min(lines.source_limits().max_work);
        let mut output = self
            .limits
            .max_output_records
            .min(lines.source_limits().max_output_declarations);
        let mut unsupported = Vec::new();
        let mut texts = Vec::new();
        let mut overlays = Vec::new();
        charge(&mut work, row.attributes().len(), "work")?;
        if row
            .attributes()
            .iter()
            .any(|a| a.origin().namespace.is_some() || a.origin().name != "id")
        {
            problem(&mut unsupported, ItemSourceProblem::UnsupportedAttribute);
        }
        if let SourceContentEvidence::Available(content) = row.content() {
            for (position, entry) in content.consumed().iter().enumerate() {
                charge(&mut work, 1, "work")?;
                match entry {
                    PobContentEntry::Text { text, .. } => texts.push((position, text.as_str())),
                    PobContentEntry::Element { child_index } => {
                        if overlays.len() >= self.limits.max_overlays {
                            return Err(ItemSourceError::Limit("overlays"));
                        }
                        let id = *row
                            .children()
                            .get(*child_index)
                            .ok_or(ItemSourceError::SourceKind)?;
                        let child = evidence.row(id)?;
                        if child.occurrence().name() != "ModRange"
                            || child.occurrence().has_namespace_context()
                            || !child.children().is_empty()
                        {
                            problem(&mut unsupported, ItemSourceProblem::UnsupportedChild);
                        }
                        overlays.push((position, child));
                    }
                }
            }
        } else {
            problem(&mut unsupported, ItemSourceProblem::ContentUnavailable);
        }
        if texts.len() != 1 || (texts.len() == 1 && overlays.iter().any(|(p, _)| *p < texts[0].0)) {
            problem(&mut unsupported, ItemSourceProblem::UnsupportedContentOrder);
        }
        if texts.len() == 1 {
            report.content_entry = Some(texts[0].0);
        }
        // Retain overlay identities even when the text lifecycle cannot be converted.
        let mut source_left = self.limits.max_source_bytes;
        for (_, text) in &texts {
            charge(&mut source_left, text.len(), "source bytes")?;
        }
        let text = texts.first().map(|(_, text)| *text).unwrap_or("");
        if text.len() > self.limits.max_source_bytes {
            return Err(ItemSourceError::Limit("source bytes"));
        }
        let mut offset = 0;
        for (i, raw) in text.split_inclusive('\n').enumerate() {
            if i >= self.limits.max_lines {
                return Err(ItemSourceError::Limit("lines"));
            }
            let stripped = raw.strip_suffix('\n').unwrap_or(raw);
            let stripped = stripped.strip_suffix('\r').unwrap_or(stripped);
            if stripped.len() > self.limits.max_line_bytes {
                return Err(ItemSourceError::Limit("line bytes"));
            }
            charge(&mut output, 1, "output records")?;
            charge(&mut work, stripped.len(), "work")?;
            report.lines.push(ItemAttributedLine {
                index: i + 1,
                decoded_span: offset..offset + stripped.len(),
                raw: stripped.into(),
                semantic_text: stripped.into(),
                rule: None,
                presentation: false,
                pending_candidates: vec![],
                member: None,
                blockers: vec![],
                range: ItemRangeDecision::Absent,
                property_tokens: vec![],
                flag_tokens: vec![],
                properties: BTreeMap::new(),
            });
            offset += raw.len();
        }
        let mut can_convert = unsupported.is_empty();
        let mut problems = Vec::new();
        let mut templates: Vec<ItemTemplateDefId> = Vec::new();
        let mut template_in_source_position = false;
        let mut catalyst_header_seen = false;
        let mut count = None;
        let mut rarity_seen = false;
        let mut started = false;
        let mut occupied_implicit = 0;
        let mut ordinals = BTreeMap::<SourceModifierCategory, usize>::new();
        let mut tag_left = self.limits.max_tags;
        let mut significant = 0usize;
        let mut item_class_first = false;
        let mut rarity_position = None;
        let mut named_rarity = false;
        let mut previous_may_combine = false;
        let mut observed_fields = BTreeSet::new();
        for line in &mut report.lines {
            let text = line.raw.trim_ascii();
            if text.is_empty() {
                continue;
            }
            significant += 1;
            let consumed_by_previous = previous_may_combine;
            previous_may_combine = false;
            if consumed_by_previous {
                problem(&mut problems, ItemSourceProblem::PossibleCombinedLine);
                problem(&mut line.blockers, ItemSourceProblem::PossibleCombinedLine);
            }
            if significant == 1 && text.starts_with("Item Class: ") {
                item_class_first = true;
            }
            let preamble = !started;
            if self.input.dialect.requires_member_proof() {
                // escapeGGGString can expose hidden headers and persistent advanced
                // controls. Do not let an unknown escaped line establish an empty-
                // tag witness for later lines. This dialect does not execute markup.
                charge(&mut work, text.len(), "work")?;
                if text.contains(['[', ']', '<', '>']) {
                    problem(
                        &mut unsupported,
                        ItemSourceProblem::UnsupportedSourceControl,
                    );
                    problem(
                        &mut line.blockers,
                        ItemSourceProblem::UnsupportedSourceControl,
                    );
                    can_convert = false;
                }
            }
            if named_rarity && rarity_position.is_some_and(|p| significant == p + 1) {
                // ParseRaw consumes the title before considering header/modifier syntax.
                line.presentation = true;
                continue;
            }
            // Only fresh source reconstruction is supported. A closed, recognized
            // prefix can prove absence of both source catalyst-setting header forms.
            // Unknown/escaped/misplaced headers leave problems and cannot prove absence.
            if self.input.dialect.requires_member_proof() {
                charge(&mut work, text.len().saturating_mul(2), "work")?;
                if text
                    .split_once(':')
                    .is_some_and(|(name, _)| name == "Catalyst" || name.contains("Quality ("))
                {
                    catalyst_header_seen = true;
                }
            }
            if text.starts_with("{ ")
                || text.strip_prefix('(').is_some_and(|rest| {
                    rest.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
                })
            {
                // Source reminder blocks consume an arbitrary span; advanced-copy
                // headers alter flags/tags on subsequent members until a delimiter.
                // Decline these lifecycles rather than replaying parser state.
                problem(
                    &mut unsupported,
                    ItemSourceProblem::UnsupportedSourceControl,
                );
                can_convert = false;
            }
            if let Some(rarity) = text.strip_prefix("Rarity: ") {
                if rarity_seen || !preamble {
                    problem(&mut problems, ItemSourceProblem::UnsupportedRarity);
                }
                if significant != if item_class_first { 2 } else { 1 } {
                    problem(&mut problems, ItemSourceProblem::UnsupportedRarity);
                }
                rarity_seen = true;
                rarity_position = Some(significant);
                named_rarity = matches!(rarity, "RARE" | "UNIQUE" | "RELIC");
                if !matches!(rarity, "NORMAL" | "MAGIC" | "RARE" | "UNIQUE" | "RELIC") {
                    problem(&mut problems, ItemSourceProblem::UnsupportedRarity);
                }
            }
            let base_position = rarity_position.map(|p| p + if named_rarity { 2 } else { 1 });
            let raw_probe =
                lines.probe_source_line(line.index, &line.raw, &mut work, &mut output)?;
            collect_candidates(
                &raw_probe,
                &mut line.pending_candidates,
                &mut output,
                &mut work,
            )?;
            let (raw_rule, raw_valid) = matched(&raw_probe);
            let raw_role = raw_rule.as_ref().and_then(|r| self.roles.get(r)).copied();
            if !self.observations.is_empty()
                && let Some(rule) = raw_rule.as_ref()
            {
                comparison_work(&mut work, rule.as_str().len(), self.observations.len())?;
                if let Some(observation) = self.observations.get(rule) {
                    let template = (template_in_source_position && templates.len() == 1)
                        .then(|| &templates[0]);
                    if let Some(template) = template {
                        comparison_work(
                            &mut work,
                            template.key().as_str().len(),
                            observation.templates.len(),
                        )?;
                    }
                    comparison_work(
                        &mut work,
                        observation.field.as_str().len(),
                        observed_fields.len(),
                    )?;
                    let duplicate = observed_fields.contains(&observation.field);
                    // The reviewed observation preamble ends at Implicits. Source can
                    // accept some later headers before its first member; this
                    // deliberately narrower boundary does not infer that lifecycle.
                    charge(&mut work, text.len(), "work")?;
                    let safe = preamble
                        && count.is_none()
                        && raw_valid
                        && can_convert
                        && problems.is_empty()
                        && line.blockers.is_empty()
                        && !text.contains(['{', '}'])
                        && !duplicate
                        && template
                            .is_some_and(|id| observation.templates.binary_search(id).is_ok());
                    if safe {
                        charge(&mut output, 1, "output records")?;
                        observed_fields.insert(observation.field.clone());
                        line.rule = raw_rule;
                        continue;
                    }
                    problem(
                        &mut line.blockers,
                        if duplicate {
                            ItemSourceProblem::DuplicatePreambleObservation
                        } else {
                            ItemSourceProblem::UnprovedPreambleObservation
                        },
                    );
                }
            }
            let header = raw_role == Some(ItemRuleSourceRole::Header);
            if header {
                previous_may_combine = consumed_by_previous;
                let metadata = if self.metadata_rules.is_empty() {
                    false
                } else if let Some(id) = raw_rule.as_ref() {
                    comparison_work(&mut work, id.as_str().len(), self.metadata_rules.len())?;
                    self.metadata_rules.contains(id)
                } else {
                    false
                };
                if metadata {
                    // These controls are pre-scanned before source header dispatch.
                    // Markup can assemble hidden control names, so it is outside
                    // this raw preamble dialect. Other brace text stays inert.
                    charge(&mut work, text.len().saturating_mul(8), "work")?;
                    let selection_control =
                        ["{variant:", "{version:", "{group:"].iter().any(|prefix| {
                            text.split_once(prefix)
                                .is_some_and(|(_, rest)| rest.contains('}'))
                        });
                    if selection_control
                        || text.contains("Foil Unique")
                        || text.contains(['[', ']', '<', '>'])
                    {
                        problem(
                            &mut unsupported,
                            ItemSourceProblem::UnsupportedSourceControl,
                        );
                        problem(
                            &mut line.blockers,
                            ItemSourceProblem::UnsupportedSourceControl,
                        );
                        can_convert = false;
                    }
                }
                line.rule = raw_rule;
                if !preamble {
                    problem(&mut problems, ItemSourceProblem::HeaderAfterModifiers);
                    problem(&mut line.blockers, ItemSourceProblem::HeaderAfterModifiers);
                }
                if !raw_valid {
                    problem(&mut problems, ItemSourceProblem::MalformedCapture);
                    problem(&mut line.blockers, ItemSourceProblem::MalformedCapture);
                }
                let is_template =
                    if let ItemLineOutcome::Known { emissions, .. } = &raw_probe.outcome {
                        let mut found = false;
                        for emission in emissions {
                            if let ConvertedItemEmission::Template { definition } = emission {
                                templates.push(definition.clone());
                                found = true;
                            }
                        }
                        found
                    } else {
                        false
                    };
                if is_template
                    && base_position == Some(significant)
                    && templates.len() == 1
                    && raw_valid
                    && problems.is_empty()
                    && line.blockers.is_empty()
                    && can_convert
                {
                    template_in_source_position = true;
                }
                let fixed_header = [
                    "Item Class: ",
                    "Rarity: ",
                    "Crafted: ",
                    "Prefix: ",
                    "Suffix: ",
                    "Item Level: ",
                    "Quality: ",
                    "Catalyst: ",
                    "CatalystQuality: ",
                    "Sockets: ",
                    "Rune: ",
                    "LevelReq: ",
                    "Implicits: ",
                ]
                .iter()
                .any(|prefix| text.starts_with(prefix));
                if is_template && base_position != Some(significant)
                    || !is_template && !fixed_header && !metadata
                {
                    problem(&mut problems, ItemSourceProblem::UnknownHeader);
                    problem(&mut line.blockers, ItemSourceProblem::UnknownHeader);
                }
                if (fixed_header || metadata)
                    && !text.starts_with("Rarity: ")
                    && !text.starts_with("Item Class: ")
                    && !base_position.is_some_and(|p| significant > p)
                {
                    problem(&mut problems, ItemSourceProblem::UnknownHeader);
                    problem(&mut line.blockers, ItemSourceProblem::UnknownHeader);
                }
                if text.starts_with("Item Class: ") && significant != 1 {
                    problem(&mut problems, ItemSourceProblem::UnknownHeader);
                    problem(&mut line.blockers, ItemSourceProblem::UnknownHeader);
                }
                if let Some(value) = text.strip_prefix("Implicits: ") {
                    if count.is_some() {
                        problem(&mut problems, ItemSourceProblem::InvalidImplicitCount);
                    }
                    count = integer(value).filter(|v| *v <= self.limits.max_lines);
                    if count.is_none() {
                        problem(&mut problems, ItemSourceProblem::InvalidImplicitCount);
                    }
                }
                if let Some(rune) = text.strip_prefix("Rune: ")
                    && rune != "None"
                {
                    problem(&mut problems, ItemSourceProblem::RuneLifecycle);
                }
                // Metadata braces are not executable modifier range instructions.
                continue;
            }
            if !base_position.is_some_and(|p| significant > p) {
                problem(&mut problems, ItemSourceProblem::UnknownHeader);
                problem(&mut line.blockers, ItemSourceProblem::UnknownHeader);
            }
            let tagged = tags(
                line,
                &self.properties,
                self.input.dialect.tracks_flags().then_some(&self.flags),
                &mut report.writes,
                &mut tag_left,
                &mut output,
                &mut work,
            )?;
            line.semantic_text = tagged.semantic;
            let probe =
                lines.probe_source_line(line.index, &line.semantic_text, &mut work, &mut output)?;
            collect_candidates(&probe, &mut line.pending_candidates, &mut output, &mut work)?;
            let (rule, valid) = matched(&probe);
            line.rule = rule.clone();
            let template = (template_in_source_position && templates.len() == 1 && can_convert)
                .then(|| &templates[0]);
            let initial_catalyst_absent = template.is_some()
                && count.is_some()
                && problems.is_empty()
                && line.blockers.is_empty()
                && !catalyst_header_seen;
            let no_modifier_tags = line.property_tokens.is_empty();
            let proof = self.proves_single_modifier(
                lines,
                (rule.as_ref(), valid),
                SourceMemberContext {
                    raw: &line.raw,
                    semantic: &line.semantic_text,
                    template,
                    scaling_syntax_safe: tagged.scaling_syntax_safe,
                    no_modifier_tags,
                    initial_catalyst_absent,
                },
                &mut work,
                &mut output,
            )?;
            let single = proof == SourceMemberProof::Single;
            // Raw and stripped paths use the same physical-line/context prerequisites.
            let raw_single = proof == SourceMemberProof::Unproved
                && self.proves_single_modifier(
                    lines,
                    (raw_rule.as_ref(), raw_valid),
                    SourceMemberContext {
                        raw: &line.raw,
                        semantic: &line.raw,
                        template,
                        scaling_syntax_safe: tagged.scaling_syntax_safe,
                        no_modifier_tags,
                        initial_catalyst_absent,
                    },
                    &mut work,
                    &mut output,
                )? == SourceMemberProof::Single;
            if proof == SourceMemberProof::FailedConditions {
                problem(
                    &mut line.blockers,
                    ItemSourceProblem::UnprovedMemberConditions,
                );
            } else if !single && self.input.dialect.requires_member_proof() {
                // V6 requires source membership independently of whether a recipe
                // happens to request property inputs. Raw recognition remains
                // available through the separate item-line conversion API.
                problem(&mut line.blockers, ItemSourceProblem::UnknownMember);
            }
            if single {
                self.bind_properties(
                    line,
                    rule.as_ref().expect("single rule"),
                    &mut work,
                    &mut output,
                )?;
            }
            // ParseRaw may consume the next physical line after a failed/partial
            // parse. A known standalone next line cannot prove it stayed independent.
            // Unconditional legacy roles and satisfied conditional roles share the
            // same decision; a raw lexical match cannot bypass a failed prerequisite.
            previous_may_combine = !(single || raw_single);
            if tagged.tagged && !single {
                problem(&mut line.blockers, ItemSourceProblem::UnprovedTaggedLine);
            }
            if !single
                || !line.blockers.is_empty()
                    && line
                        .blockers
                        .iter()
                        .any(|p| !matches!(p, ItemSourceProblem::InvalidRange))
            {
                problem(
                    &mut problems,
                    if !valid && rule.is_some() {
                        ItemSourceProblem::MalformedCapture
                    } else {
                        ItemSourceProblem::UnknownMember
                    },
                );
                if preamble {
                    problem(&mut problems, ItemSourceProblem::UnknownHeader);
                    problem(&mut line.blockers, ItemSourceProblem::UnknownHeader);
                }
                continue;
            }
            started = true;
            if !rarity_seen {
                problem(&mut problems, ItemSourceProblem::MissingRarity);
            }
            let category = if tagged.enchant {
                SourceModifierCategory::Enchant
            } else if tagged.implicit || occupied_implicit < count.unwrap_or(0) {
                SourceModifierCategory::Implicit
            } else {
                SourceModifierCategory::Explicit
            };
            if matches!(
                category,
                SourceModifierCategory::Enchant | SourceModifierCategory::Implicit
            ) {
                occupied_implicit += 1;
            }
            let ordinal = ordinals.entry(category).or_default();
            *ordinal += 1;
            line.member = Some(SourceModifierSlot {
                category,
                ordinal: *ordinal,
                line: line.index,
            });
        }
        if !rarity_seen {
            problem(&mut problems, ItemSourceProblem::MissingRarity);
        }
        if count.is_none() {
            problem(&mut problems, ItemSourceProblem::MissingImplicitCount);
        }
        if templates.len() != 1
            || self.prefixes.get(&templates[0])
                != Some(&ItemLoadIndexPrefix::NoGeneratedBuffMembers)
        {
            problem(&mut problems, ItemSourceProblem::UnknownTemplatePrefix);
        }
        let proven = can_convert && problems.is_empty();
        let mut ordered: Vec<_> = report.lines.iter().filter_map(|l| l.member).collect();
        ordered.sort_by_key(|slot| (slot.category, slot.ordinal));
        if !proven {
            for line in &mut report.lines {
                if !matches!(line.range, ItemRangeDecision::Absent) {
                    line.range = ItemRangeDecision::Pending;
                }
            }
        }
        for (position, child) in overlays {
            charge(&mut work, child.attributes().len() + 1, "work")?;
            charge(&mut output, 1, "output records")?;
            for attribute in child.attributes() {
                charge(&mut work, attribute.raw().len().saturating_mul(4), "work")?;
            }
            let (id_ref, id_text) = field(child, "id");
            let (range_ref, range_text) = field(child, "range");
            let id = id_text.and_then(integer).filter(|v| *v > 0);
            let value = range_text.and_then(fraction);
            let valid = child.occurrence().name() == "ModRange"
                && !child.occurrence().has_namespace_context()
                && child.attributes().len() == 2
                && child.attributes().iter().all(|a| {
                    a.origin().namespace.is_none()
                        && matches!(a.origin().name.as_str(), "id" | "range")
                })
                && id.is_some()
                && value.is_some();
            let target = if proven && valid {
                id.and_then(|v| ordered.get(v - 1))
                    .map_or(ItemRangeTarget::IgnoredOutOfBounds, |slot| {
                        ItemRangeTarget::Line(slot.line)
                    })
            } else {
                ItemRangeTarget::Pending
            };
            let write = report.writes.len();
            report.writes.push(ItemRangeWrite {
                origin: ItemRangeOrigin::Xml {
                    occurrence: child.occurrence().id(),
                    content_entry: position,
                    id: id_ref,
                    range: range_ref,
                },
                source_id: id,
                fraction: value,
                target: target.clone(),
            });
            match target {
                ItemRangeTarget::Line(index) => {
                    report.lines[index - 1].range = ItemRangeDecision::Resolved {
                        fraction: value.expect("validated fraction"),
                        winning_write: write,
                    };
                }
                ItemRangeTarget::Pending => {
                    // Unknown targets cannot leave an earlier fraction authoritative.
                    charge(&mut work, report.lines.len(), "work")?;
                    for line in &mut report.lines {
                        line.range = ItemRangeDecision::Pending;
                    }
                    if !valid {
                        problem(&mut problems, ItemSourceProblem::InvalidOverlay);
                    }
                }
                ItemRangeTarget::IgnoredOutOfBounds => {}
            }
        }
        report.layout = if !can_convert {
            ItemLayoutStatus::Unsupported(unsupported)
        } else if problems.is_empty() {
            ItemLayoutStatus::Proven
        } else {
            ItemLayoutStatus::Pending(problems)
        };
        let defaults = self.prepare_defaults(&mut report, &templates, &mut work, &mut output)?;
        Ok(ItemRangeAttribution {
            defaults,
            report,
            work_left: work,
            output_left: output,
            can_convert,
        })
    }
}
