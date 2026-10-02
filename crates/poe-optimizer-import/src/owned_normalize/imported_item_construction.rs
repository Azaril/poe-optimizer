//! Opt-in proof of fresh imported item construction. External header syntax is
//! confined to Import; no source names or template IDs enter the native runtime.
use super::equipment_membership::{CompiledEquipmentMembership, EquipmentAugmentBase};
use super::*;
use crate::owned_value::{DecimalSyntax, ValueCodecInput, WhitespacePolicy};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportedItemConstructionProfile {
    pub template: ItemTemplateDefId,
    pub headers: Vec<ImportedItemHeader>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportedItemHeader {
    pub field: ImportedItemHeaderField,
    pub rule: OwnedDefinitionKey,
    pub capture: OwnedDefinitionKey,
    pub cardinality: ImportedHeaderCardinality,
    pub value: ImportedHeaderValue,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportedItemHeaderField {
    Rarity,
    UniqueId,
    ItemLevel,
    RequirementLevel,
    ImplicitCount,
}
impl ImportedItemHeaderField {
    fn prefix(self) -> &'static str {
        match self {
            Self::Rarity => "Rarity: ",
            Self::UniqueId => "Unique ID: ",
            Self::ItemLevel => "Item Level: ",
            Self::RequirementLevel => "LevelReq: ",
            Self::ImplicitCount => "Implicits: ",
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportedHeaderCardinality {
    RequiredOnce,
    OptionalOnce,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ImportedHeaderValue {
    Literal { value: String },
    CanonicalUnsigned { maximum: u64 },
    LowerHex { bytes: usize },
}

pub(super) fn validate_profile(
    profile: &ImportedItemConstructionProfile,
    limits: NormalizationLimits,
) -> Result<usize> {
    if profile.headers.len() != 5 {
        return Err(NormalizationError::Policy("imported item header domain"));
    }
    let mut fields = BTreeSet::new();
    let mut rules = BTreeSet::new();
    let mut work = profile
        .template
        .key()
        .as_str()
        .len()
        .saturating_add(profile.template.namespace().game().as_str().len())
        .saturating_add(profile.template.namespace().version().as_str().len());
    for header in &profile.headers {
        let literal_bytes = match &header.value {
            ImportedHeaderValue::Literal { value } => value.len(),
            _ => 0,
        };
        work = work
            .checked_add(
                header
                    .rule
                    .as_str()
                    .len()
                    .saturating_add(header.capture.as_str().len())
                    .saturating_add(literal_bytes)
                    .saturating_add(1),
            )
            .filter(|v| *v <= limits.max_work)
            .ok_or(NormalizationError::Limit("imported item policy work"))?;
        let valid = match (&header.field, &header.value) {
            (ImportedItemHeaderField::Rarity, ImportedHeaderValue::Literal { value }) => {
                value == "RARE"
            }
            (ImportedItemHeaderField::UniqueId, ImportedHeaderValue::LowerHex { bytes }) => {
                (1..=128).contains(bytes)
            }
            (
                ImportedItemHeaderField::ItemLevel,
                ImportedHeaderValue::CanonicalUnsigned { maximum },
            ) => *maximum <= u64::from(u16::MAX),
            (
                ImportedItemHeaderField::RequirementLevel,
                ImportedHeaderValue::CanonicalUnsigned { maximum },
            ) => *maximum <= 9_007_199_254_740_991,
            (
                ImportedItemHeaderField::ImplicitCount,
                ImportedHeaderValue::CanonicalUnsigned { maximum },
            ) => *maximum <= 64,
            _ => false,
        };
        if !valid
            || !fields.insert(header.field)
            || !rules.insert(&header.rule)
            || matches!(
                header.field,
                ImportedItemHeaderField::Rarity
                    | ImportedItemHeaderField::UniqueId
                    | ImportedItemHeaderField::ImplicitCount
            ) && header.cardinality != ImportedHeaderCardinality::RequiredOnce
        {
            return Err(NormalizationError::Policy(
                "imported item header constraint",
            ));
        }
    }
    if profile.headers[0].field != ImportedItemHeaderField::Rarity {
        return Err(NormalizationError::Policy("imported item rarity framing"));
    }
    Ok(work)
}

pub(super) fn validate_bindings(
    policy: Option<&EquipmentMembershipPolicy>,
    items: &OwnedItemLinePolicy,
    source: &ItemSourceLayoutPolicy,
    limits: NormalizationLimits,
) -> Result<()> {
    let Some(
        EquipmentMembershipPolicy::PobOrdinaryAndImportedItemSetsV2 {
            item_lines,
            item_source,
            imported_profiles,
            ..
        }
        | EquipmentMembershipPolicy::PobOrdinaryImportedAndEmptyCharacterRunesV3 {
            item_lines,
            item_source,
            imported_profiles,
            ..
        },
    ) = policy
    else {
        return Ok(());
    };
    if item_lines != items.identity() || item_source != source.identity() {
        return Err(NormalizationError::Binding);
    }
    let mut work = 0usize;
    for profile in imported_profiles {
        for header in &profile.headers {
            work = work
                .checked_add(
                    items
                        .input()
                        .rules
                        .len()
                        .saturating_add(source.input().rule_layouts.len())
                        .saturating_add(source.input().dialect.metadata_rules().len())
                        .saturating_add(1),
                )
                .filter(|v| *v <= limits.max_work)
                .ok_or(NormalizationError::Limit("imported item binding work"))?;
            let Some(rule) = items.input().rules.iter().find(|r| r.id == header.rule) else {
                return Err(NormalizationError::Policy("imported item header rule"));
            };
            let prefix = header.field.prefix();
            let grammar = if header.field == ImportedItemHeaderField::ItemLevel {
                matches!(rule.pattern.as_slice(), [ItemPatternPart::Literal(p), ItemPatternPart::NumericCapture { capture, syntax: DecimalSyntax::Integer, sign: ItemNumericSign::Forbidden }] if p == prefix && capture == &header.capture)
                    && matches!(rule.captures.as_slice(), [ItemCapture { id, codec: ItemCaptureCodec::Value(ValueCodecInput { namespace, whitespace: WhitespacePolicy::Exact, codec: ValueCodecKind::Integer { syntax: DecimalSyntax::Integer } }) }] if id == &header.capture && namespace == &items.input().namespace)
                    && matches!(rule.emissions.as_slice(), [ItemEmission::ItemLevel { value: ItemLineValue::Capture(capture) }] if capture == &header.capture)
            } else {
                matches!(rule.pattern.as_slice(), [ItemPatternPart::Literal(p), ItemPatternPart::Capture(capture)] if p == prefix && capture == &header.capture)
                    && matches!(rule.captures.as_slice(), [ItemCapture { id, codec: ItemCaptureCodec::OpaqueText }] if id == &header.capture)
                    && matches!(rule.emissions.as_slice(), [ItemEmission::Metadata { .. }])
            };
            if !grammar
                || (header.field == ImportedItemHeaderField::UniqueId
                    && !source
                        .input()
                        .dialect
                        .metadata_rules()
                        .contains(&header.rule))
                || !source
                    .input()
                    .rule_layouts
                    .iter()
                    .any(|l| l.rule == header.rule && l.role == ItemRuleSourceRole::Header)
            {
                return Err(NormalizationError::Policy(
                    "imported item header source semantics",
                ));
            }
        }
    }
    Ok(())
}

#[derive(Clone)]
pub(super) struct ImportedConstructionEvidence {
    source: SourceOccurrenceId,
    profile: ImportedItemConstructionProfile,
    item_lines: OwnedContentDigest,
    item_source: OwnedContentDigest,
    headers: BTreeMap<usize, OwnedDefinitionKey>,
}
impl ImportedConstructionEvidence {
    pub(super) fn matches(
        &self,
        b: &Builder<'_, '_>,
        source: SourceOccurrenceId,
        profile: &ImportedItemConstructionProfile,
    ) -> bool {
        self.source == source
            && &self.profile == profile
            && &self.item_lines == b.items.identity()
            && &self.item_source == b.item_source.identity()
    }
    pub(super) fn header(&self, line: usize, rule: &OwnedDefinitionKey) -> bool {
        self.headers.get(&line) == Some(rule)
    }
}
fn accepts(raw: &str, value: &ImportedHeaderValue) -> bool {
    match value {
        ImportedHeaderValue::Literal { value } => raw == value,
        ImportedHeaderValue::CanonicalUnsigned { maximum } => {
            !raw.is_empty()
                && (raw.len() == 1 || !raw.starts_with('0'))
                && raw.bytes().all(|c| c.is_ascii_digit())
                && raw.parse::<u64>().is_ok_and(|v| v <= *maximum)
        }
        ImportedHeaderValue::LowerHex { bytes } => {
            raw.len() == *bytes
                && raw
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        }
    }
}
fn forbidden(text: &str) -> bool {
    text.contains(['{', '}', '<', '>', '[', ']'])
        || matches!(
            text,
            "Unidentified"
                | "Corrupted"
                | "Twice Corrupted"
                | "Mirrored"
                | "Sanctified"
                | "Desecrated Prefix"
                | "Desecrated Suffix"
                | "--------"
        )
        || text.starts_with("Crafted:")
        || text.starts_with("Prefix:")
        || text.starts_with("Suffix:")
        || text
            .strip_prefix('(')
            .is_some_and(|r| r.as_bytes().first().is_some_and(u8::is_ascii_alphabetic))
        || text.split(" (").skip(1).any(|tail| {
            tail.split_once(')').is_some_and(|(flag, _)| {
                !flag.is_empty() && flag.bytes().all(|v| v.is_ascii_lowercase())
            })
        })
}
pub(super) fn prove(
    b: &mut Builder<'_, '_>,
    source: SourceOccurrenceId,
    base: &EquipmentAugmentBase,
    policy: &CompiledEquipmentMembership<'_>,
    profile: &ImportedItemConstructionProfile,
    raw: &str,
) -> Result<Option<ImportedConstructionEvidence>> {
    b.charge(raw.len().saturating_add(profile.headers.len()))?;
    if base.weapon
        || base.armour
        || base.wand
        || base.staff
        || base.sceptre
        || profile.template != base.template
        || raw
            .chars()
            .any(|c| c.is_whitespace() && !c.is_ascii_whitespace())
    {
        return Ok(None);
    }
    let framing: Vec<_> = raw
        .lines()
        .map(str::trim_ascii)
        .filter(|v| !v.is_empty())
        .take(3)
        .collect();
    if framing.len() != 3
        || framing[0] != "Rarity: RARE"
        || framing[2] != base.base_name
        || framing[1].contains(':')
        || forbidden(framing[1])
        || policy.loader_jewel_fallback_titles.contains(framing[1])
    {
        return Ok(None);
    }
    let attribution = b.item_source.attribute(b.evidence, source, b.items)?;
    let report = attribution.report();
    b.charge(report.lines.len().saturating_add(report.writes.len()))?;
    if !matches!(report.layout, ItemLayoutStatus::Proven) || report.item != source {
        return Ok(None);
    }
    let mut targets = BTreeSet::new();
    for write in &report.writes {
        b.charge(report.lines.len())?;
        if !matches!(
            &write.origin,
            crate::owned_item_source::ItemRangeOrigin::Xml { .. }
        ) || !write.source_id.is_some_and(|id| targets.insert(id))
            || !matches!(&write.target, crate::owned_item_source::ItemRangeTarget::Line(index) if report.lines.iter().any(|line| line.index == *index && line.member.is_some()))
            || !write
                .fraction
                .is_some_and(|v| v.is_finite() && (0.0..=1.0).contains(&v))
        {
            return Ok(None);
        }
    }
    let mut headers = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut last = None;
    let mut member_started = false;
    for line in &report.lines {
        b.charge(
            line.raw
                .len()
                .saturating_add(profile.headers.len())
                .saturating_add(1),
        )?;
        let text = line.raw.trim_ascii();
        if forbidden(text) || !line.blockers.is_empty() {
            return Ok(None);
        }
        if text.is_empty() || line.presentation {
            continue;
        }
        if text == base.base_name {
            if line.index != 3 {
                return Ok(None);
            }
            continue;
        }
        if policy
            .source_base_names
            .contains(text.strip_prefix("Superior ").unwrap_or(text))
        {
            return Ok(None);
        }
        if line.member.is_some() {
            member_started = true;
            continue;
        }
        if member_started {
            return Ok(None);
        }
        let Some(rule) = &line.rule else {
            return Ok(None);
        };
        let Some((index, header)) = profile
            .headers
            .iter()
            .enumerate()
            .find(|(_, h)| &h.rule == rule)
        else {
            return Ok(None);
        };
        if !seen.insert(index) || last.is_some_and(|old| index < old) {
            return Ok(None);
        }
        last = Some(index);
        let Some(value) = text.strip_prefix(header.field.prefix()) else {
            return Ok(None);
        };
        if !accepts(value, &header.value) {
            return Ok(None);
        }
        b.charge(rule.as_str().len())?;
        headers.insert(line.index, rule.clone());
    }
    if profile.headers.iter().enumerate().any(|(i, h)| {
        h.cardinality == ImportedHeaderCardinality::RequiredOnce && !seen.contains(&i)
    }) {
        return Ok(None);
    }
    b.charge(validate_profile(profile, b.limits)?)?;
    Ok(Some(ImportedConstructionEvidence {
        source,
        profile: profile.clone(),
        item_lines: *b.items.identity(),
        item_source: *b.item_source.identity(),
        headers,
    }))
}
