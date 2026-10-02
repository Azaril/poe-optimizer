//! Finite ordinary ItemSet membership, independent of item modifier conversion.
//!
//! This is a deliberately narrow PoB source grammar. It proves empty augment
//! inventories; it neither emulates ParseRaw nor admits occupied socket uses.
use super::source_shape::{charge_row, container_text, plain_row, retire_membership, value};
use super::*;
use crate::source_xml::PobContentEntry;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EquipmentMembershipPolicy {
    PobOrdinaryItemSetsV1 {
        definitions: DataIdentity,
        templates: Vec<EquipmentAugmentBase>,
        /// Complete source base-name recognition inventory (keys and recognized
        /// aliases). Detects replacements without a reviewed template row.
        source_base_names: Vec<String>,
        /// Complete ItemsTab.Load title-key inventory for implicit jewel sockets.
        loader_jewel_fallback_titles: Vec<String>,
    },
    /// Explicit fresh imported construction, bound to reviewed source rules.
    /// Templates without a profile retain the historical ordinary grammar.
    PobOrdinaryAndImportedItemSetsV2 {
        definitions: DataIdentity,
        templates: Vec<EquipmentAugmentBase>,
        source_base_names: Vec<String>,
        loader_jewel_fallback_titles: Vec<String>,
        item_lines: OwnedContentDigest,
        item_source: OwnedContentDigest,
        imported_profiles: Vec<ImportedItemConstructionProfile>,
    },
    /// Adds an independently reviewed explicit-empty character-rune source lane.
    /// Occupied rune effects and ordinary ItemSet completeness remain separate.
    PobOrdinaryImportedAndEmptyCharacterRunesV3 {
        definitions: DataIdentity,
        templates: Vec<EquipmentAugmentBase>,
        source_base_names: Vec<String>,
        loader_jewel_fallback_titles: Vec<String>,
        item_lines: OwnedContentDigest,
        item_source: OwnedContentDigest,
        imported_profiles: Vec<ImportedItemConstructionProfile>,
        empty_character_runes: EmptyCharacterRuneSelections,
    },
}

impl EquipmentMembershipPolicy {
    pub(crate) fn definitions_mut(&mut self) -> &mut DataIdentity {
        match self {
            Self::PobOrdinaryItemSetsV1 { definitions, .. }
            | Self::PobOrdinaryAndImportedItemSetsV2 { definitions, .. }
            | Self::PobOrdinaryImportedAndEmptyCharacterRunesV3 { definitions, .. } => definitions,
        }
    }
    pub(crate) fn imported_bindings_mut(
        &mut self,
    ) -> Option<(&mut OwnedContentDigest, &mut OwnedContentDigest)> {
        match self {
            Self::PobOrdinaryItemSetsV1 { .. } => None,
            Self::PobOrdinaryAndImportedItemSetsV2 {
                item_lines,
                item_source,
                ..
            }
            | Self::PobOrdinaryImportedAndEmptyCharacterRunesV3 {
                item_lines,
                item_source,
                ..
            } => Some((item_lines, item_source)),
        }
    }
    pub(super) fn templates(&self) -> &[EquipmentAugmentBase] {
        match self {
            Self::PobOrdinaryItemSetsV1 { templates, .. }
            | Self::PobOrdinaryAndImportedItemSetsV2 { templates, .. }
            | Self::PobOrdinaryImportedAndEmptyCharacterRunesV3 { templates, .. } => templates,
        }
    }
    pub(super) fn imported_profile(
        &self,
        template: &ItemTemplateDefId,
    ) -> Option<&ImportedItemConstructionProfile> {
        match self {
            Self::PobOrdinaryItemSetsV1 { .. } => None,
            Self::PobOrdinaryAndImportedItemSetsV2 {
                imported_profiles, ..
            }
            | Self::PobOrdinaryImportedAndEmptyCharacterRunesV3 {
                imported_profiles, ..
            } => imported_profiles.iter().find(|p| &p.template == template),
        }
    }
}

/// Dependency identity for adapters that reuse the complete source base-name
/// inventory. It commits the exact supplied equipment policy, including profiles.
pub fn equipment_membership_identity(
    policy: &EquipmentMembershipPolicy,
    limits: NormalizationLimits,
) -> Result<OwnedContentDigest> {
    Ok(digest_owned(
        "owned-equipment-membership-policy-v1",
        policy,
        limits.max_policy_bytes.min(MAX_NORMALIZATION_POLICY_BYTES),
    )?)
}

/// Reviewed immutable source-base facts, never a build-specific empty flag.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EquipmentAugmentBase {
    pub template: ItemTemplateDefId,
    pub base_name: String,
    pub weapon: bool,
    pub armour: bool,
    pub wand: bool,
    pub staff: bool,
    pub sceptre: bool,
}

pub(super) struct CompiledEquipmentMembership<'p> {
    bases: BTreeMap<&'p ItemTemplateDefId, &'p EquipmentAugmentBase>,
    pub(super) source_base_names: BTreeSet<&'p str>,
    pub(super) loader_jewel_fallback_titles: BTreeSet<&'p str>,
    imported_profiles: BTreeMap<&'p ItemTemplateDefId, &'p ImportedItemConstructionProfile>,
    pub work: usize,
}

pub(super) fn compile<'p, I: DefinitionSchemaIndex>(
    policy: Option<&'p EquipmentMembershipPolicy>,
    definitions: &I,
    limits: NormalizationLimits,
) -> Result<Option<CompiledEquipmentMembership<'p>>> {
    let Some(policy) = policy else {
        return Ok(None);
    };
    let (identity, templates, source_base_names, loader_jewel_fallback_titles, imported_profiles) =
        match policy {
            EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 {
                definitions: identity,
                templates,
                source_base_names,
                loader_jewel_fallback_titles,
            } => (
                identity,
                templates,
                source_base_names,
                loader_jewel_fallback_titles,
                &[][..],
            ),
            EquipmentMembershipPolicy::PobOrdinaryAndImportedItemSetsV2 {
                definitions: identity,
                templates,
                source_base_names,
                loader_jewel_fallback_titles,
                imported_profiles,
                ..
            }
            | EquipmentMembershipPolicy::PobOrdinaryImportedAndEmptyCharacterRunesV3 {
                definitions: identity,
                templates,
                source_base_names,
                loader_jewel_fallback_titles,
                imported_profiles,
                ..
            } => (
                identity,
                templates,
                source_base_names,
                loader_jewel_fallback_titles,
                imported_profiles.as_slice(),
            ),
        };
    if identity != definitions.identity() {
        return Err(NormalizationError::Binding);
    }
    if templates.len() > 4096 || imported_profiles.len() > 4096 {
        return Err(NormalizationError::Policy("equipment augment base count"));
    }
    let mut work = 0usize;
    let mut inventory = |rows: &'p [String], maximum: usize| -> Result<BTreeSet<&'p str>> {
        if rows.len() > maximum {
            return Err(NormalizationError::Policy(
                "equipment source inventory count",
            ));
        }
        let mut result = BTreeSet::new();
        for name in rows {
            work = work
                .checked_add(name.len().saturating_add(1))
                .filter(|v| *v <= limits.max_work)
                .ok_or(NormalizationError::Limit(
                    "equipment membership policy work",
                ))?;
            if name.is_empty()
                || name.len() > limits.mapping.max_string_bytes
                || name.trim_ascii() != name
                || name.contains(['\n', '\r'])
                || !result.insert(name.as_str())
            {
                return Err(NormalizationError::Policy("equipment source inventory"));
            }
        }
        Ok(result)
    };
    let source_base_names = inventory(source_base_names, 16384)?;
    let loader_jewel_fallback_titles = inventory(loader_jewel_fallback_titles, 256)?;
    let mut bases = BTreeMap::new();
    let mut names = BTreeSet::new();
    for base in templates {
        work = work
            .checked_add(base.base_name.len().saturating_add(1))
            .filter(|work| *work <= limits.max_work)
            .ok_or(NormalizationError::Limit(
                "equipment membership policy work",
            ))?;
        if base.base_name.is_empty()
            || base.base_name.len() > limits.mapping.max_string_bytes
            || base.base_name.trim_ascii() != base.base_name
            || base
                .base_name
                .contains(['\n', '\r', '{', '}', '<', '>', '[', ']'])
            || !source_base_names.contains(base.base_name.as_str())
            || !matches!(
                definitions.definition(&base.template),
                SchemaLookup::Known(_)
            )
            || bases.insert(&base.template, base).is_some()
            || !names.insert(&base.base_name)
        {
            return Err(NormalizationError::Policy("equipment augment base facts"));
        }
    }
    let mut profiles = BTreeMap::new();
    for profile in imported_profiles {
        work = work
            .checked_add(imported_item_construction::validate_profile(
                profile, limits,
            )?)
            .filter(|v| *v <= limits.max_work)
            .ok_or(NormalizationError::Limit("imported item policy work"))?;
        let Some(base) = bases.get(&profile.template) else {
            return Err(NormalizationError::Policy("imported item template base"));
        };
        if profile.template.namespace() != definitions.namespace()
            || base.weapon
            || base.armour
            || base.wand
            || base.staff
            || base.sceptre
            || profiles.insert(&profile.template, profile).is_some()
        {
            return Err(NormalizationError::Policy("imported item profile domain"));
        }
    }
    Ok(Some(CompiledEquipmentMembership {
        bases,
        source_base_names,
        loader_jewel_fallback_titles,
        imported_profiles: profiles,
        work,
    }))
}

fn decimal_id(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|v| v.is_ascii_digit())
        && !value.starts_with('0')
        && value.parse::<u32>().is_ok_and(|v| v > 0)
}

fn same_unique_key(evidence: &SourceProjectEvidence<'_>, row: &SourceEvidenceRow<'_>) -> bool {
    let Some(id) = value(row, "id").filter(|v| decimal_id(v)) else {
        return false;
    };
    matches!(
        evidence.lookup_key(SourceKeyQuery {
            parent: row.occurrence().parent(),
            element: SourceQName { namespace: None, local: row.occurrence().name() },
            attribute: SourceQName { namespace: None, local: "id" },
            value: id,
        }),
        Ok(SourceKeyLookup::Unique(found)) if found.occurrence == row.occurrence().id()
    )
}

/// Only tags with no branch/augment-membership effect are admitted. Scan all tags,
/// including those embedded in crafted Prefix/Suffix metadata. A malformed tag
/// cannot hide a socket header or a source rune/variant annotation.
fn harmless_tags(line: &str) -> bool {
    let mut rest = line;
    while let Some(open) = rest.find('{') {
        if rest[..open].contains('}') {
            return false;
        }
        let Some(close) = rest[open + 1..].find('}') else {
            return false;
        };
        let tag = &rest[open + 1..open + 1 + close];
        let harmless = if let Some(number) = tag.strip_prefix("range:") {
            number
                .parse::<f64>()
                .is_ok_and(|v| v.is_finite() && (0.0..=1.0).contains(&v))
        } else if let Some(tags) = tag.strip_prefix("tags:") {
            !tags.is_empty()
                && tags
                    .bytes()
                    .all(|v| v.is_ascii_lowercase() || matches!(v, b'_' | b','))
        } else {
            false
        };
        if !harmless {
            return false;
        }
        rest = &rest[open + close + 2..];
    }
    !rest.contains('}')
}

fn empty_raw_augments(
    raw: &str,
    base: &EquipmentAugmentBase,
    policy: &CompiledEquipmentMembership<'_>,
) -> Option<usize> {
    if raw
        .chars()
        .any(|v| v.is_whitespace() && !v.is_ascii_whitespace())
    {
        return None;
    }
    let mut lines = raw
        .lines()
        .map(str::trim_ascii)
        .filter(|line| !line.is_empty());
    if lines.next() != Some("Rarity: RARE") {
        return None;
    }
    let title = lines.next()?;
    if title.contains(['{', '}', ':', '<', '>', '[', ']'])
        || title == "--------"
        || title == "Unidentified"
        || policy.loader_jewel_fallback_titles.contains(title)
    {
        return None;
    }
    if lines.next() != Some(base.base_name.as_str()) {
        return None;
    }
    let mut sockets = None;
    let mut runes = 0usize;
    for line in lines {
        if line.contains(['<', '>', '[', ']']) || line == "Unidentified" || !harmless_tags(line) {
            return None;
        }
        // Source strips tags and lowercase parenthetical line flags before base
        // lookup. Reject the latter entirely; strip only already-proved tags.
        if line.split(" (").skip(1).any(|tail| {
            tail.split_once(')').is_some_and(|(flag, _)| {
                !flag.is_empty() && flag.bytes().all(|v| v.is_ascii_lowercase())
            })
        }) {
            return None;
        }
        let without_tags: String = line
            .split('{')
            .enumerate()
            .map(|(index, part)| {
                if index == 0 {
                    part
                } else {
                    part.split_once('}').expect("validated tag").1
                }
            })
            .collect();
        let candidate = without_tags
            .strip_prefix("Superior ")
            .unwrap_or(&without_tags);
        if policy.source_base_names.contains(candidate) {
            return None;
        }
        // Source reminder blocks can consume subsequent apparent headers. This
        // grammar does not attempt to reconstruct that state machine.
        if line
            .strip_prefix('(')
            .is_some_and(|tail| tail.as_bytes().first().is_some_and(u8::is_ascii_alphabetic))
        {
            return None;
        }
        if let Some(shape) = line.strip_prefix("Sockets: ") {
            if sockets.is_some()
                || shape.is_empty()
                || !shape.bytes().all(|v| matches!(v, b'S' | b' '))
            {
                return None;
            }
            let count = shape.bytes().filter(|v| *v == b'S').count();
            if count == 0 || shape.split(' ').any(|v| v != "S") {
                return None;
            }
            sockets = Some(count);
        } else if line == "Rune: None" {
            runes += 1;
        } else if let Some((name, _)) = line.split_once(": ").filter(|(name, value)| {
            !value.is_empty()
                && name
                    .bytes()
                    .all(|v| v.is_ascii_alphabetic() || matches!(v, b' ' | b'(' | b')' | b':'))
        }) {
            // These specifications cannot change socket membership. Remaining
            // ordinary modifier text is retained but is not evaluated here.
            if !matches!(
                name,
                "Armour"
                    | "Energy Shield"
                    | "Evasion"
                    | "Evasion Rating"
                    | "Ward"
                    | "Runic Ward"
                    | "Crafted"
                    | "Prefix"
                    | "Suffix"
                    | "Quality"
                    | "LevelReq"
                    | "Implicits"
                    | "Charm Slots"
                    | "Spirit"
                    | "Item Level"
                    | "Requires Level"
                    | "Grants Skill"
            ) {
                return None;
            }
        }
    }
    match sockets {
        Some(count) => (runes == count).then_some(count),
        None => (runes == 0
            && !(base.weapon || base.armour || base.wand || base.staff || base.sceptre))
            .then_some(0),
    }
}

fn empty_item(
    b: &mut Builder<'_, '_>,
    source: SourceOccurrenceId,
    item: &ItemDraft,
    policy: &CompiledEquipmentMembership<'_>,
) -> Result<bool> {
    let template = match &item.template {
        DraftField::Known { value } => Some(value),
        DraftField::Pending(_) => None,
    };
    fresh_empty_item_with_template(b, source, template, policy).map(|proof| proof.is_some())
}

pub(super) struct FreshEmptyItem {
    pub socket_capacity: usize,
    pub imported: Option<imported_item_construction::ImportedConstructionEvidence>,
}

/// Private fresh per-occurrence proof shared with modifier inventory admission.
pub(super) fn fresh_empty_item_for_template(
    b: &mut Builder<'_, '_>,
    source: SourceOccurrenceId,
    template: &ItemTemplateDefId,
    policy: &CompiledEquipmentMembership<'_>,
) -> Result<Option<FreshEmptyItem>> {
    fresh_empty_item_with_template(b, source, Some(template), policy)
}

fn fresh_empty_item_with_template(
    b: &mut Builder<'_, '_>,
    source: SourceOccurrenceId,
    template: Option<&ItemTemplateDefId>,
    policy: &CompiledEquipmentMembership<'_>,
) -> Result<Option<FreshEmptyItem>> {
    let evidence = b.evidence;
    let row = &evidence.rows()[source.ordinal() as usize];
    charge_row(b, row)?;
    if !plain_row(row, &["id"], false) || !same_unique_key(evidence, row) {
        return Ok(None);
    }
    let Some(template) = template else {
        return Ok(None);
    };
    let Some(base) = policy.bases.get(template) else {
        return Ok(None);
    };
    let SourceContentEvidence::Available(content) = row.content() else {
        return Ok(None);
    };
    // ItemsTab.Load calls ParseRaw on one text entry. Multiple nonempty entries,
    // arbitrary overlays or nested objects have no empty-inventory proof here.
    let mut raw = None;
    for entry in content.consumed() {
        if let PobContentEntry::Text { text, .. } = entry
            && !text.trim_ascii().is_empty()
            && raw.replace(text.as_str()).is_some()
        {
            return Ok(None);
        }
    }
    for child in row.children() {
        let child = &evidence.rows()[child.ordinal() as usize];
        charge_row(b, child)?;
        if child.occurrence().name() != "ModRange"
            || !plain_row(child, &["id", "range"], true)
            || !value(child, "id").is_some_and(decimal_id)
            || !value(child, "range").is_some_and(|v| {
                v.parse::<f64>()
                    .is_ok_and(|v| v.is_finite() && (0.0..=1.0).contains(&v))
            })
        {
            return Ok(None);
        }
    }
    if let Some(profile) = policy.imported_profiles.get(template) {
        let Some(raw) = raw else {
            return Ok(None);
        };
        return Ok(
            imported_item_construction::prove(b, source, base, policy, profile, raw)?.map(
                |imported| FreshEmptyItem {
                    socket_capacity: 0,
                    imported: Some(imported),
                },
            ),
        );
    }
    Ok(raw
        .and_then(|raw| empty_raw_augments(raw, base, policy))
        .map(|socket_capacity| FreshEmptyItem {
            socket_capacity,
            imported: None,
        }))
}

pub(super) fn boolean_token(value: &str) -> bool {
    matches!(value, "true" | "false" | "nil")
}

/// Loader item IDs are numbers, not lexical XML keys. Requiring canonical
/// positive decimal identifiers throughout this one container prevents a later
/// `01` record from replacing an earlier `1` despite an exact-string lookup.
pub(super) fn ordinary_items(b: &mut Builder<'_, '_>) -> Result<Option<SourceOccurrenceId>> {
    b.charge(1)?;
    if let Some(proof) = b.ordinary_items_source {
        return Ok(proof);
    }
    let proof = prove_ordinary_items(b)?;
    b.ordinary_items_source = Some(proof);
    Ok(proof)
}

fn prove_ordinary_items(b: &mut Builder<'_, '_>) -> Result<Option<SourceOccurrenceId>> {
    let evidence = b.evidence;
    let [source] = evidence.sections(SourceSectionKind::Items) else {
        return Ok(None);
    };
    let row = &evidence.rows()[source.ordinal() as usize];
    charge_row(b, row)?;
    let root = &evidence.rows()[0];
    b.charge(root.children().len())?;
    // The recognized section index deliberately omits namespace contexts. Such
    // a sibling still prevents proving one unambiguous source loader invocation.
    if root
        .children()
        .iter()
        .filter(|child| {
            evidence.rows()[child.ordinal() as usize]
                .occurrence()
                .name()
                == "Items"
        })
        .count()
        != 1
    {
        return Ok(None);
    }
    if row.occurrence().parent() != Some(evidence.rows()[0].occurrence().id())
        || evidence.rows()[0].occurrence().name() != "PathOfBuilding2"
        || evidence.rows()[0].occurrence().has_namespace_context()
        || !plain_row(
            row,
            &["activeItemSet", "showStatDifferences", "useSecondWeaponSet"],
            false,
        )
        || !container_text(row)
        || value(row, "activeItemSet").is_some_and(|v| !decimal_id(v))
        || ["showStatDifferences", "useSecondWeaponSet"]
            .iter()
            .any(|name| value(row, name).is_some_and(|v| !boolean_token(v)))
    {
        return Ok(None);
    }
    let mut item_keys = BTreeSet::new();
    let mut set_keys = BTreeSet::new();
    for child in row.children() {
        let child = &evidence.rows()[child.ordinal() as usize];
        charge_row(b, child)?;
        if child.occurrence().has_namespace_context() {
            return Ok(None);
        }
        let keys = match child.occurrence().name() {
            "Item" => &mut item_keys,
            "ItemSet" => &mut set_keys,
            // ItemsTab.Load consumes this separate branch only for UI trade
            // sorting. Its flat Stat rows cannot alter item or receiving state.
            "TradeSearchWeights" => {
                if !plain_row(child, &[], false) || !container_text(child) {
                    return Ok(None);
                }
                for stat in child.children() {
                    let stat = &evidence.rows()[stat.ordinal() as usize];
                    charge_row(b, stat)?;
                    if stat.occurrence().name() != "Stat"
                        || !plain_row(stat, &["label", "stat", "weightMult"], true)
                        || value(stat, "label").is_none()
                        || value(stat, "stat").is_none()
                        || !value(stat, "weightMult")
                            .is_some_and(|v| v.parse::<f64>().is_ok_and(f64::is_finite))
                    {
                        return Ok(None);
                    }
                }
                continue;
            }
            _ => return Ok(None),
        };
        // Other item internals are checked only if a particular set uses them;
        // archived occupied items must not poison an independent set's proof.
        let mut ids = child
            .attributes()
            .iter()
            .filter(|a| a.origin().name == "id");
        let Some(id) = ids
            .next()
            .filter(|a| a.origin().namespace.is_none())
            .and_then(|a| a.decoded().ok())
        else {
            return Ok(None);
        };
        if ids.next().is_some() || !decimal_id(id) || !keys.insert(id) {
            return Ok(None);
        }
    }
    Ok(Some(*source))
}

pub(super) fn close(
    b: &mut Builder<'_, '_>,
    draft: &mut DraftSessionInput,
    policy: &CompiledEquipmentMembership<'_>,
    rules: &BTreeMap<&str, &EquipmentLoadoutRule>,
    sets: &BTreeMap<SourceOccurrenceId, usize>,
    item_ids: &BTreeMap<SourceOccurrenceId, ItemRecordId>,
) -> Result<()> {
    let Some(items_parent) = ordinary_items(b)? else {
        return Ok(());
    };
    b.charge(
        draft
            .items
            .members
            .len()
            .saturating_add(draft.equipment.members.len()),
    )?;
    let items: BTreeMap<_, _> = draft
        .items
        .members
        .iter()
        .map(|item| (item.id, item))
        .collect();
    let uses: BTreeMap<_, _> = draft
        .equipment
        .members
        .iter()
        .map(|item| (item.id, item))
        .collect();
    let evidence = b.evidence;
    let mut empty_items = BTreeMap::new();
    for (source, index) in sets {
        let row = &evidence.rows()[source.ordinal() as usize];
        charge_row(b, row)?;
        let Some(parent) = row.occurrence().parent() else {
            continue;
        };
        if parent != items_parent
            || !plain_row(row, &["id", "title", "useSecondWeaponSet"], false)
            || !container_text(row)
            || !same_unique_key(evidence, row)
            || value(row, "useSecondWeaponSet").is_some_and(|v| !boolean_token(v))
        {
            continue;
        }
        let mut names = BTreeSet::new();
        let mut url_nodes = BTreeSet::new();
        let mut expected = Vec::new();
        let mut proven = true;
        for child in row.children() {
            let child = &evidence.rows()[child.ordinal() as usize];
            charge_row(b, child)?;
            if child.occurrence().name() == "SocketIdURL" {
                // The saved display name is ignored by ItemsTab.Load; nodeId
                // remains the sole URL-table key and never an Item reference.
                if !plain_row(child, &["nodeId", "itemPbURL", "name"], true)
                    || !value(child, "nodeId").is_some_and(|v| decimal_id(v) && url_nodes.insert(v))
                    || value(child, "itemPbURL").is_none()
                {
                    proven = false;
                    break;
                }
                continue;
            }
            if child.occurrence().name() != "Slot"
                || !plain_row(
                    child,
                    &["name", "itemId", "active", "itemPbURL", "note"],
                    true,
                )
                || value(child, "active").is_some_and(|v| !boolean_token(v))
            {
                proven = false;
                break;
            }
            let Some(name) = value(child, "name") else {
                proven = false;
                break;
            };
            let Some(rule) = rules.get(name) else {
                proven = false;
                break;
            };
            if !names.insert(name) {
                proven = false;
                break;
            }
            let Some(item_key) = value(child, "itemId") else {
                proven = false;
                break;
            };
            let origin_index = child.occurrence().id().ordinal() as usize;
            b.charge(b.origins[origin_index].links.len())?;
            let origin = &b.origins[origin_index];
            if matches!(&origin.disposition, SourceDisposition::SourceOnly(code) if code.as_str() == "explicit-empty-equipment-use")
            {
                if item_key != "0" {
                    proven = false;
                    break;
                }
                continue;
            }
            if !decimal_id(item_key) {
                proven = false;
                break;
            }
            let Ok(SourceKeyLookup::Unique(join)) = evidence.lookup_key(SourceKeyQuery {
                parent: Some(parent),
                element: SourceQName {
                    namespace: None,
                    local: "Item",
                },
                attribute: SourceQName {
                    namespace: None,
                    local: "id",
                },
                value: item_key,
            }) else {
                proven = false;
                break;
            };
            let Some(item_id) = item_ids.get(&join.occurrence) else {
                proven = false;
                break;
            };
            let mut linked_uses = origin.links.iter().filter_map(|link| match link {
                OwnedOriginTarget::Equipment(id) => Some(*id),
                _ => None,
            });
            let Some(use_id) = linked_uses.next() else {
                proven = false;
                break;
            };
            if linked_uses.next().is_some() {
                proven = false;
                break;
            }
            let Some(receiving) = uses.get(&use_id) else {
                proven = false;
                break;
            };
            if !matches!(&receiving.item, DraftField::Known { value } if value == item_id)
                || !matches!(&receiving.destination, DraftEquipmentDestination::CharacterSlot(DraftField::Known {value}) if value == &rule.destination)
                || !matches!(&receiving.scope, DraftField::Known { .. })
            {
                proven = false;
                break;
            }
            let is_empty = if let Some(empty) = empty_items.get(item_id) {
                *empty
            } else {
                let Some(item) = items.get(item_id) else {
                    proven = false;
                    break;
                };
                let empty = empty_item(b, join.occurrence, item, policy)?;
                empty_items.insert(*item_id, empty);
                empty
            };
            if !is_empty {
                proven = false;
                break;
            }
            if expected.len() >= b.limits.draft.input.max_collection_entries {
                return Err(NormalizationError::Limit("equipment preset membership"));
            }
            expected.push(use_id);
        }
        let preset = &mut draft.equipment_presets.members[*index];
        b.charge(expected.len())?;
        if !proven || expected != preset.equipment.members {
            continue;
        }
        retire_membership(
            b,
            *source,
            &mut preset.equipment.completion,
            "equipment-membership-not-converted",
        )?;
    }
    Ok(())
}
