//! A finite fresh-source proof for physical item inputs. Static declarations and
//! rule coverage remain independent, including every retained Partial gap.
use super::*;
use crate::owned_value::{OwnedValueCodec, ValueCodecInput, ValueDecodeError};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ItemParameterInputsPolicy {
    PobFreshOrdinaryInputsV1 {
        definitions: DataIdentity,
        item_lines: OwnedContentDigest,
        item_source: OwnedContentDigest,
        templates: Vec<OrdinaryItemParameterInputs>,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrdinaryItemParameterInputs {
    pub template: ItemTemplateDefId,
    pub header_inputs: Vec<ItemParameterHeaderInput>,
    pub corruption_slot: DeclaredSlot<ParameterSlotDefId>,
    pub capacity_slot: DeclaredSlot<ParameterSlotDefId>,
    pub construction: OrdinaryItemConstruction,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemParameterHeaderInput {
    pub rule: OwnedDefinitionKey,
    pub capture: OwnedDefinitionKey,
    pub codec: ValueCodecInput,
    pub slot: DeclaredSlot<ParameterSlotDefId>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrdinaryItemConstruction {
    FreshRareSavedAffixesV1,
    FreshRareSavedImplicitExplicitV2,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemParameterInputEvidence {
    pub slot: DeclaredSlot<ParameterSlotDefId>,
    pub value: ParameterValue,
    pub origin: ItemParameterInputOrigin,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ItemParameterInputOrigin {
    Header {
        line: usize,
        rule: OwnedDefinitionKey,
        capture: OwnedDefinitionKey,
    },
    FreshUncorrupted,
    EmptySocketCapacity {
        line: usize,
    },
    /// The checked fresh base/augment proof establishes zero capacity without
    /// any socket header. This is not a fabricated source line.
    AbsentSocketHeader,
}

pub(super) struct CompiledItemParameterInputs<'p> {
    templates: BTreeMap<&'p ItemTemplateDefId, BoundTemplate<'p>>,
    pub work: usize,
}
struct BoundTemplate<'p> {
    input: &'p OrdinaryItemParameterInputs,
    declared: BTreeMap<DeclaredSlot<ParameterSlotDefId>, ParameterSlotSchema>,
    projected: BTreeSet<&'p DeclaredSlot<ParameterSlotDefId>>,
    headers: Vec<BoundHeader<'p>>,
    observations: BTreeSet<OwnedDefinitionKey>,
}
struct BoundHeader<'p> {
    input: &'p ItemParameterHeaderInput,
    codec: OwnedValueCodec,
    rule_index: usize,
}
pub(super) struct PhysicalItemInputs {
    pub parameters: Vec<ParameterAssignment>,
    pub evidence: Vec<ItemParameterInputEvidence>,
}
fn charge(work: &mut usize, n: usize, limits: NormalizationLimits) -> Result<()> {
    *work = work
        .checked_add(n)
        .filter(|v| *v <= limits.max_work)
        .ok_or(NormalizationError::Limit("item parameter inputs work"))?;
    Ok(())
}
fn invalid<T>(why: &'static str) -> Result<T> {
    Err(NormalizationError::Policy(why))
}
fn value_fits(value: &ParameterValue, schema: &ValueSchema) -> bool {
    match (value, schema) {
        (ParameterValue::Boolean(_), ValueSchema::Boolean) => true,
        (ParameterValue::Integer(v), ValueSchema::Integer(r)) => v >= &r.minimum && v <= &r.maximum,
        (ParameterValue::Quantity(v), ValueSchema::Quantity(r)) => {
            v.unit() == r.minimum.unit()
                && v.value() >= r.minimum.value()
                && v.value() <= r.maximum.value()
        }
        (ParameterValue::Option(v), ValueSchema::Option { allowed }) => allowed.members.contains(v),
        _ => false,
    }
}

pub(super) fn validate_base<'p, I: DefinitionSchemaIndex>(
    policy: &'p NormalizationPolicy,
    definitions: &I,
    limits: NormalizationLimits,
) -> Result<Option<CompiledItemParameterInputs<'p>>> {
    let Some(ItemParameterInputsPolicy::PobFreshOrdinaryInputsV1 {
        definitions: identity,
        templates,
        ..
    }) = &policy.item_parameter_inputs
    else {
        return Ok(None);
    };
    if identity != definitions.identity() {
        return Err(NormalizationError::Binding);
    }
    if templates.len() > 4096 {
        return Err(NormalizationError::Limit("item parameter templates"));
    }
    let Some(membership) = &policy.item_modifier_membership else {
        return invalid("item parameter inputs require item modifier membership proof");
    };
    let mut result = CompiledItemParameterInputs {
        templates: BTreeMap::new(),
        work: 0,
    };
    for row in templates {
        charge(
            &mut result.work,
            membership
                .template_count()
                .saturating_add(row.template.key().as_str().len())
                .saturating_add(1),
            limits,
        )?;
        if row.header_inputs.len() != 2
            || row.template.namespace() != definitions.namespace()
            || !membership.admits_construction(
                &row.template,
                row.construction == OrdinaryItemConstruction::FreshRareSavedImplicitExplicitV2,
            )
            || result.templates.contains_key(&row.template)
        {
            return invalid("item parameter input template domain");
        }
        let SchemaLookup::Known(template) = definitions.definition(&row.template) else {
            return invalid("item parameter template schema");
        };
        charge(
            &mut result.work,
            template.declarations.parameters.members.len(),
            limits,
        )?;
        let mut declared = BTreeMap::new();
        for slot in &template.declarations.parameters.members {
            let SchemaLookup::Known(schema) = definitions.slot(slot) else {
                return invalid("item parameter declaration schema");
            };
            let value_work = match &schema.value {
                ValueSchema::Option { allowed } => allowed.members.len(),
                _ => 0,
            };
            charge(
                &mut result.work,
                slot.slot
                    .key()
                    .as_str()
                    .len()
                    .saturating_add(schema.sites.len())
                    .saturating_add(value_work)
                    .saturating_add(1),
                limits,
            )?;
            if slot.declaration != SlotOwnerDefId::ItemTemplate(row.template.clone())
                || !schema.sites.contains(&ParameterSite::ItemParameter)
                || declared.insert(slot.clone(), schema.clone()).is_some()
            {
                return invalid("item parameter declaration ownership");
            }
        }
        let mut projected = BTreeSet::new();
        for slot in row
            .header_inputs
            .iter()
            .map(|v| &v.slot)
            .chain([&row.corruption_slot, &row.capacity_slot])
        {
            charge(
                &mut result.work,
                slot.slot.key().as_str().len().saturating_add(1),
                limits,
            )?;
            if !declared.contains_key(slot) || !projected.insert(slot) {
                return invalid("item parameter projected slot");
            }
        }
        let corruption = &declared[&row.corruption_slot];
        let capacity = &declared[&row.capacity_slot];
        if corruption.value != ValueSchema::Boolean
            || corruption.presence != SlotPresence::RequiredOnce
            || !matches!(capacity.value, ValueSchema::Integer(_))
            || capacity.presence != SlotPresence::RequiredOnce
        {
            return invalid("item parameter corruption or capacity type");
        }
        let mut headers = Vec::new();
        let mut rules = BTreeSet::new();
        let mut kinds = (0, 0);
        for input in &row.header_inputs {
            charge(
                &mut result.work,
                input
                    .rule
                    .as_str()
                    .len()
                    .saturating_add(input.capture.as_str().len())
                    .saturating_add(1),
                limits,
            )?;
            if input.codec.namespace != *definitions.namespace() || !rules.insert(&input.rule) {
                return invalid("item parameter header namespace or duplicate");
            }
            let schema = &declared[&input.slot];
            match (&input.codec.codec, &schema.value) {
                (
                    ValueCodecKind::Integer {
                        syntax: crate::owned_value::DecimalSyntax::Integer,
                    },
                    ValueSchema::Integer(_),
                ) => {
                    kinds.1 += 1;
                }
                (ValueCodecKind::Option { tokens }, ValueSchema::Option { allowed }) => {
                    charge(
                        &mut result.work,
                        tokens.iter().fold(0usize, |sum, t| {
                            sum.saturating_add(t.token.len())
                                .saturating_add(t.value.key().as_str().len())
                                .saturating_add(allowed.members.len())
                                .saturating_add(1)
                        }),
                        limits,
                    )?;
                    if schema.presence != SlotPresence::RequiredOnce
                        || tokens.is_empty()
                        || tokens.iter().any(|t| {
                            !allowed.members.contains(&t.value)
                                || !matches!(
                                    definitions.definition(&t.value),
                                    SchemaLookup::Known(_)
                                )
                        })
                    {
                        return invalid("item parameter option domain");
                    }
                    kinds.0 += 1;
                }
                _ => return invalid("item parameter codec schema"),
            }
            let codec = OwnedValueCodec::new(input.codec.clone(), limits.value.value)
                .map_err(ItemLineError::from)?;
            headers.push(BoundHeader {
                input,
                codec,
                rule_index: usize::MAX,
            });
        }
        if kinds != (1, 1) {
            return invalid("item parameter header kinds");
        }
        result.templates.insert(
            &row.template,
            BoundTemplate {
                input: row,
                declared,
                projected,
                headers,
                observations: BTreeSet::new(),
            },
        );
    }
    Ok(Some(result))
}

pub(super) fn compile<'p, I: DefinitionSchemaIndex>(
    policy: &'p NormalizationPolicy,
    definitions: &I,
    items: &OwnedItemLinePolicy,
    source: &ItemSourceLayoutPolicy,
    limits: NormalizationLimits,
) -> Result<Option<CompiledItemParameterInputs<'p>>> {
    let Some(mut result) = validate_base(policy, definitions, limits)? else {
        return Ok(None);
    };
    let Some(ItemParameterInputsPolicy::PobFreshOrdinaryInputsV1 {
        item_lines,
        item_source,
        ..
    }) = &policy.item_parameter_inputs
    else {
        unreachable!()
    };
    if item_lines != items.identity() || item_source != source.identity() {
        return Err(NormalizationError::Binding);
    }
    let observations = match &source.input().dialect {
        ItemSourceDialect::PobExportedSingleTextObservationsV1 {
            preamble_observations,
            ..
        }
        | ItemSourceDialect::PobExportedSingleTextCategoriesV1 {
            preamble_observations,
            ..
        } => preamble_observations.as_slice(),
        _ => &[],
    };
    for row in result.templates.values_mut() {
        for header in &mut row.headers {
            charge(
                &mut result.work,
                items
                    .input()
                    .rules
                    .len()
                    .saturating_add(source.input().rule_layouts.len()),
                limits,
            )?;
            let Some((index, rule)) = items
                .input()
                .rules
                .iter()
                .enumerate()
                .find(|(_, r)| r.id == header.input.rule)
            else {
                return invalid("item parameter header rule");
            };
            charge(
                &mut result.work,
                rule.pattern
                    .len()
                    .saturating_add(rule.captures.len())
                    .saturating_add(rule.emissions.len()),
                limits,
            )?;
            let prefix = if matches!(header.input.codec.codec, ValueCodecKind::Option { .. }) {
                "Rarity: "
            } else {
                "LevelReq: "
            };
            if !matches!(rule.pattern.as_slice(), [ItemPatternPart::Literal(p), ItemPatternPart::Capture(c)] if p==prefix && c==&header.input.capture)
                || !matches!(rule.captures.as_slice(), [ItemCapture {id, codec:ItemCaptureCodec::OpaqueText}] if id==&header.input.capture)
                || !matches!(rule.emissions.as_slice(), [ItemEmission::Metadata { .. }])
                || !source
                    .input()
                    .rule_layouts
                    .iter()
                    .any(|r| r.rule == rule.id && r.role == ItemRuleSourceRole::Header)
            {
                return invalid("item parameter header source grammar");
            }
            header.rule_index = index;
        }
        for observation in observations {
            charge(
                &mut result.work,
                observation
                    .templates
                    .len()
                    .saturating_add(observation.rule.as_str().len())
                    .saturating_add(1),
                limits,
            )?;
            if observation.templates.contains(&row.input.template) {
                row.observations.insert(observation.rule.clone());
            }
        }
    }
    Ok(Some(result))
}

impl CompiledItemParameterInputs<'_> {
    pub(super) fn permits_missing_template_parameters(
        &self,
        b: &mut Builder<'_, '_>,
        converted: &ItemTextConversion<'_>,
    ) -> Result<bool> {
        b.charge(1)?;
        let ItemField::Known {
            value: template, ..
        } = &converted.template
        else {
            return Ok(false);
        };
        let Some(row) = self.templates.get(template) else {
            return Ok(false);
        };
        let Some(present) = present_slots(b, row, converted)? else {
            return Ok(false);
        };
        b.charge(row.declared.len())?;
        Ok(row.declared.iter().all(|(slot, schema)| {
            schema.presence != SlotPresence::RequiredOnce
                || present.contains(slot)
                || row.projected.contains(slot)
        }))
    }
    pub(super) fn prove(
        &self,
        b: &mut Builder<'_, '_>,
        source_row: &SourceEvidenceRow<'_>,
        attribution: &ItemRangeAttribution,
        converted: &ItemTextConversion<'_>,
        modifier: Option<&item_modifier_membership::ModifierMembershipProof>,
    ) -> Result<Option<PhysicalItemInputs>> {
        b.charge(1)?;
        let source = source_row.occurrence().id();
        let ItemField::Known {
            value: template, ..
        } = &converted.template
        else {
            return Ok(None);
        };
        let Some(row) = self.templates.get(template) else {
            return Ok(None);
        };
        let paired =
            row.input.construction == OrdinaryItemConstruction::FreshRareSavedImplicitExplicitV2;
        let Some(capacity) = modifier.and_then(|p| p.capacity_for(source, paired)) else {
            return Ok(None);
        };
        if attribution.report().item != source
            || !matches!(attribution.report().layout, ItemLayoutStatus::Proven)
        {
            return Ok(None);
        }
        b.charge(converted.issues.len())?;
        if converted.issues.iter().any(|i| {
            !i.lines.is_empty()
                || !matches!(
                    i.problem,
                    ItemTextProblem::SchemaPartial | ItemTextProblem::RequiredParameterMissing
                )
        }) {
            return Ok(None);
        }
        let Some(mut present) = present_slots(b, row, converted)? else {
            return Ok(None);
        };
        // The earlier converter may not already supply any of the new projections.
        if row.projected.iter().any(|s| present.contains(*s)) {
            return Ok(None);
        }
        let mut parameters = Vec::new();
        let mut evidence = Vec::new();
        let mut seen = BTreeSet::new();
        let (mut crafted, mut prefixes, mut suffixes, mut sockets, mut implicit) =
            (0, 0, 0, None, 0);
        for line in &attribution.report().lines {
            b.charge(
                line.raw
                    .len()
                    .saturating_add(line.blockers.len())
                    .saturating_add(1),
            )?;
            if !line.blockers.is_empty() {
                return Ok(None);
            }
            let text = line.raw.trim_ascii();
            // Some state setters are scanned independently of apparent source
            // presentation. An injected observation cannot erase their effect.
            if matches!(
                text,
                "Unidentified"
                    | "Corrupted"
                    | "Twice Corrupted"
                    | "Mirrored"
                    | "Sanctified"
                    | "Desecrated Prefix"
                    | "Desecrated Suffix"
            ) {
                return Ok(None);
            }
            if line.presentation {
                continue;
            }
            if line.member.is_some() {
                continue;
            } // already sealed by the exact physical-member witness
            let Some(rule) = &line.rule else {
                return Ok(None);
            };
            b.charge(converted.lines.len())?;
            let Some(converted_line) = converted.lines.iter().find(|l| l.index == line.index)
            else {
                return Ok(None);
            };
            let ItemLineOutcome::Known {
                rule: converted_rule,
                emissions,
            } = &converted_line.outcome
            else {
                return Ok(None);
            };
            if converted_rule != rule {
                return Ok(None);
            }
            if let Some(header) = row.headers.iter().find(|h| &h.input.rule == rule) {
                if !seen.insert(rule) {
                    return Ok(None);
                }
                let remaining = b.limits.max_work.saturating_sub(b.work);
                let mut work = remaining;
                let mut output = remaining;
                let captures = b.items.source_rule_captures(
                    header.rule_index,
                    &line.raw,
                    &mut work,
                    &mut output,
                );
                b.charge(
                    remaining
                        .saturating_sub(work)
                        .saturating_add(remaining.saturating_sub(output)),
                )?;
                let Some(captures) = captures? else {
                    return Ok(None);
                };
                let Some(raw) = captures.get(&header.input.capture) else {
                    return Ok(None);
                };
                b.charge(raw.len().saturating_add(1))?;
                let value = match header.codec.decode(raw) {
                    Ok(v) => v,
                    Err(ValueDecodeError::SourceTooLarge { .. }) => {
                        return Err(NormalizationError::Limit("item parameter capture bytes"));
                    }
                    Err(_) => return Ok(None),
                };
                if !value_fits(&value, &row.declared[&header.input.slot].value) {
                    return Ok(None);
                }
                b.charge(
                    header
                        .input
                        .slot
                        .slot
                        .key()
                        .as_str()
                        .len()
                        .saturating_add(rule.as_str().len())
                        .saturating_add(header.input.capture.as_str().len())
                        .saturating_add(2),
                )?;
                parameters.push(ParameterAssignment {
                    slot: header.input.slot.clone(),
                    value: value.clone(),
                });
                evidence.push(ItemParameterInputEvidence {
                    slot: header.input.slot.clone(),
                    value,
                    origin: ItemParameterInputOrigin::Header {
                        line: line.index,
                        rule: rule.clone(),
                        capture: header.input.capture.clone(),
                    },
                });
                continue;
            }
            // Typed existing conversions account for these values; display fields
            // require their separate exact-template, checked preamble declaration.
            let existing_input = match emissions.as_slice() {
                [ConvertedItemEmission::Template { definition }] => definition == template,
                [ConvertedItemEmission::ItemLevel { .. }] => text.starts_with("Item Level: "),
                [ConvertedItemEmission::Quality { .. }] => text.starts_with("Quality: "),
                [
                    ConvertedItemEmission::ItemParameter { .. }
                    | ConvertedItemEmission::TemplateParameter { .. },
                ] => {
                    b.charge(converted.parameters.len())?;
                    (text.starts_with("Catalyst: ") || text.starts_with("CatalystQuality: "))
                        && converted.parameters.iter().any(|p| p.line == line.index)
                }
                _ => false,
            };
            if existing_input {
                continue;
            }
            if row.observations.contains(rule)
                && matches!(
                    emissions.as_slice(),
                    [ConvertedItemEmission::Metadata { .. }]
                )
                && ["Armour: ", "Energy Shield: ", "Evasion: "]
                    .iter()
                    .any(|prefix| text.starts_with(prefix))
            {
                continue;
            }
            if !matches!(
                emissions.as_slice(),
                [ConvertedItemEmission::Metadata { .. }]
            ) {
                return Ok(None);
            }
            if text.starts_with("Crafted: ") {
                crafted += 1;
            } else if text.starts_with("Prefix: ") {
                prefixes += 1;
            } else if text.starts_with("Suffix: ") {
                suffixes += 1;
            } else if text.starts_with("Sockets: ") {
                if sockets.replace(line.index).is_some() {
                    return Ok(None);
                }
            } else if text == "Rune: None" { /* exact count/content was sealed by fresh proof */
            } else if text
                == if paired {
                    "Implicits: 1"
                } else {
                    "Implicits: 0"
                }
            {
                implicit += 1;
            } else {
                return Ok(None);
            }
        }
        if seen.len() != 2 || crafted != 1 || prefixes != 3 || suffixes != 3 || implicit != 1 {
            return Ok(None);
        }
        let capacity_origin = match sockets {
            Some(line) => ItemParameterInputOrigin::EmptySocketCapacity { line },
            None if paired && capacity == 0 => ItemParameterInputOrigin::AbsentSocketHeader,
            None => return Ok(None),
        };
        let capacity = i64::try_from(capacity)
            .ok()
            .and_then(|n| BoundedInteger::new(n).ok());
        let Some(capacity) = capacity else {
            return Ok(None);
        };
        let values = [
            (
                &row.input.corruption_slot,
                ParameterValue::Boolean(false),
                ItemParameterInputOrigin::FreshUncorrupted,
            ),
            (
                &row.input.capacity_slot,
                ParameterValue::Integer(capacity),
                capacity_origin,
            ),
        ];
        for (slot, value, origin) in values {
            if !value_fits(&value, &row.declared[slot].value) {
                return Ok(None);
            }
            b.charge(slot.slot.key().as_str().len().saturating_add(2))?;
            parameters.push(ParameterAssignment {
                slot: slot.clone(),
                value: value.clone(),
            });
            evidence.push(ItemParameterInputEvidence {
                slot: slot.clone(),
                value,
                origin,
            });
        }
        b.charge(row.declared.len().saturating_add(parameters.len()))?;
        present.extend(parameters.iter().map(|p| &p.slot));
        if present.len() != row.declared.len() || !row.declared.keys().all(|s| present.contains(s))
        {
            return Ok(None);
        }
        Ok(Some(PhysicalItemInputs {
            parameters,
            evidence,
        }))
    }
}
fn present_slots<'a>(
    b: &mut Builder<'_, '_>,
    row: &BoundTemplate<'_>,
    converted: &'a ItemTextConversion<'_>,
) -> Result<Option<BTreeSet<&'a DeclaredSlot<ParameterSlotDefId>>>> {
    let count = converted
        .parameters
        .len()
        .saturating_add(converted.defaults.parameters.len());
    b.charge(count)?;
    let mut present = BTreeSet::new();
    for assignment in converted
        .parameters
        .iter()
        .map(|p| &p.assignment)
        .chain(&converted.defaults.parameters)
    {
        b.charge(assignment.slot.slot.key().as_str().len().saturating_add(1))?;
        let Some(schema) = row.declared.get(&assignment.slot) else {
            return Ok(None);
        };
        if !value_fits(&assignment.value, &schema.value) || !present.insert(&assignment.slot) {
            return Ok(None);
        }
    }
    Ok(Some(present))
}
