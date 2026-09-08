//! Strict, source-preserving admission for the reviewed local Mace item families.
//! Understanding supplied rolls does not establish affix, roll-tier or acquisition legality.
use poe_optimizer_data::game_data::{
    ActorModifierRecord, ActorStat, GameDataPackage, ItemCaptureKind, ItemModifierRoll,
    ItemModifierRule,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::ops::Range;
use thiserror::Error;

pub const MAX_MACE_ITEM_BYTES: usize = 8 * 1024;
pub const MAX_MACE_MODIFIER_LINES: usize = 64;
const MAX_LINE_BYTES: usize = 256;
const MAX_ROLL: f64 = 1_000_000.0;

#[derive(Debug, Error)]
#[error("unsupported local Mace item: {0}")]
pub struct MaceItemError(String);
fn invalid(message: impl Into<String>) -> MaceItemError {
    MaceItemError(message.into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum MaceItemRarity {
    Normal,
    Rare,
}

/// Location and exact authored spelling of one interpreted modifier line.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MaceModifierLine {
    /// One-based line number in the exact supplied item payload, including blank lines.
    pub line_number: usize,
    /// Bytes of the literal line, excluding its line ending but retaining edge whitespace.
    pub byte_range: Range<usize>,
    pub source: String,
    pub rule_id: String,
    pub values: Vec<f64>,
    pub effective_values: Vec<f64>,
}

/// Immutable values admitted from a supplied payload against one selected data package.
/// The numeric engine independently validates rule identities and operations at compilation.
#[derive(Debug, Clone)]
pub struct ValidatedMaceWeapon {
    weapon_key: String,
    base_name: String,
    rarity: MaceItemRarity,
    rare_name: Option<String>,
    quality: u32,
    item_level: u32,
    explicit_level_requirement: Option<u32>,
    effective_level_requirement: u32,
    source_text: String,
    source_sha256: String,
    local_modifiers: Vec<ItemModifierRoll>,
    modifier_lines: Vec<MaceModifierLine>,
    actor_modifiers: Vec<ActorModifierRecord>,
}
impl ValidatedMaceWeapon {
    pub fn weapon_key(&self) -> &str {
        &self.weapon_key
    }
    pub fn base_name(&self) -> &str {
        &self.base_name
    }
    pub fn rarity(&self) -> MaceItemRarity {
        self.rarity
    }
    pub fn rare_name(&self) -> Option<&str> {
        self.rare_name.as_deref()
    }
    pub fn quality(&self) -> u32 {
        self.quality
    }
    pub fn item_level(&self) -> u32 {
        self.item_level
    }
    pub fn explicit_level_requirement(&self) -> Option<u32> {
        self.explicit_level_requirement
    }
    /// PoB's non-unique item path uses an authored LevelReq if present, otherwise base level.
    /// Item level remains unrelated; skill/support and attribute checks belong to the domain.
    pub fn effective_level_requirement(&self) -> u32 {
        self.effective_level_requirement
    }
    pub fn source_text(&self) -> &str {
        &self.source_text
    }
    pub fn source_sha256(&self) -> &str {
        &self.source_sha256
    }
    pub fn local_modifiers(&self) -> &[ItemModifierRoll] {
        &self.local_modifiers
    }
    pub fn modifier_lines(&self) -> &[MaceModifierLine] {
        &self.modifier_lines
    }
    /// Global actor records remaining after reviewed weapon-local consumption.
    pub fn actor_modifiers(&self) -> &[ActorModifierRecord] {
        &self.actor_modifiers
    }
    /// Historical schemas require the former exact five-line normal-item grammar.
    pub fn is_legacy_normal_payload(&self) -> bool {
        if self.rarity != MaceItemRarity::Normal
            || self.explicit_level_requirement.is_some()
            || !self.local_modifiers.is_empty()
            || !self.actor_modifiers.is_empty()
            || self.source_text.len() > 1024
        {
            return false;
        }
        let text = self.source_text.replace("\r\n", "\n");
        let lines: Vec<_> = text.trim().lines().collect();
        lines.len() == 5
            && lines[0] == "Rarity: NORMAL"
            && lines[1] == self.base_name
            && lines[2]
                .strip_prefix("Item Level: ")
                .is_some_and(|n| n == self.item_level.to_string())
            && lines[3]
                .strip_prefix("Quality: ")
                .is_some_and(|n| n == self.quality.to_string())
            && lines[4] == "Implicits: 0"
    }
    pub fn diagnostic(&self) -> serde_json::Value {
        serde_json::json!({
            "schema_version":1,"source_sha256":self.source_sha256,
            "rarity":self.rarity,"rare_name":self.rare_name,"weapon_key":self.weapon_key,
            "base_name":self.base_name,"quality":self.quality,"item_level":self.item_level,
            "explicit_level_requirement":self.explicit_level_requirement,
            "effective_level_requirement":self.effective_level_requirement,
            "modifier_lines":self.modifier_lines,"local_modifiers":self.local_modifiers,
            "actor_modifiers":self.actor_modifiers,
            "affix_legality_verified":false,
        })
    }
    /// Narrow normalization performed by the pinned reference Item:BuildRaw path.
    /// Fixed modifier lines retain order and spelling; only metadata is regenerated.
    pub(crate) fn pob_export_lines(&self) -> Vec<String> {
        let mut lines = vec![
            match self.rarity {
                MaceItemRarity::Normal => "Rarity: NORMAL",
                MaceItemRarity::Rare => "Rarity: RARE",
            }
            .into(),
        ];
        if let Some(name) = &self.rare_name {
            lines.push(name.clone());
        }
        lines.extend([
            self.base_name.clone(),
            format!("Item Level: {}", self.item_level),
            format!("Quality: {}", self.quality),
            format!("LevelReq: {}", self.effective_level_requirement),
            "Implicits: 0".into(),
        ]);
        lines.extend(
            self.modifier_lines
                .iter()
                .map(|line| line.source.trim_ascii().to_owned()),
        );
        lines
    }
}
struct SourceLine<'a> {
    number: usize,
    range: Range<usize>,
    raw: &'a str,
    trimmed: &'a str,
}
fn unsigned(value: &str, min: u32, max: u32, name: &str) -> Result<u32, MaceItemError> {
    if value.is_empty()
        || !value.bytes().all(|b| b.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return Err(invalid(format!(
            "{name} requires canonical unsigned integer syntax"
        )));
    }
    value
        .parse::<u32>()
        .ok()
        .filter(|v| (min..=max).contains(v))
        .ok_or_else(|| invalid(format!("{name} is outside {min}..={max}")))
}
fn header<'a>(line: Option<&'a SourceLine<'_>>, prefix: &str) -> Result<&'a str, MaceItemError> {
    line.and_then(|line| line.trimmed.strip_prefix(prefix))
        .ok_or_else(|| invalid(format!("expected ordered item field {prefix}")))
}
fn match_rule(line: &str, rule: &ItemModifierRule) -> Result<Option<Vec<f64>>, MaceItemError> {
    let literals = rule
        .template_literals()
        .map_err(|e| invalid(e.to_string()))?;
    let Some(mut rest) = line.strip_prefix(literals[0]) else {
        return Ok(None);
    };
    let mut values = Vec::with_capacity(rule.captures.len());
    for (index, kind) in rule.captures.iter().enumerate() {
        let next = literals[index + 1];
        let (number, remaining) = if next.is_empty() {
            (rest, "")
        } else {
            let Some((number, remaining)) = rest.split_once(next) else {
                return Ok(None);
            };
            (number, remaining)
        };
        let syntax = match kind {
            ItemCaptureKind::UnsignedInteger => {
                !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit())
            }
            ItemCaptureKind::UnsignedDecimal => {
                let mut parts = number.split('.');
                let integral = parts.next().unwrap_or("");
                !integral.is_empty()
                    && integral.bytes().all(|b| b.is_ascii_digit())
                    && parts.next().is_none_or(|fraction| {
                        !fraction.is_empty() && fraction.bytes().all(|b| b.is_ascii_digit())
                    })
                    && parts.next().is_none()
            }
        };
        if !syntax {
            return Ok(None);
        }
        let Some(number) = number
            .parse::<f64>()
            .ok()
            .filter(|n| n.is_finite() && (0.0..=MAX_ROLL).contains(n))
        else {
            return Ok(None);
        };
        values.push(number);
        rest = remaining;
    }
    if !rest.is_empty() {
        return Ok(None);
    }
    if values.len() == 2 && values[0] > values[1] {
        return Err(invalid("local flat damage minimum exceeds maximum"));
    }
    Ok(Some(values))
}

/// Read one unsplit Item payload before XML line-ending normalization. Only the named
/// entities understood by PoB are decoded; literal CRLF and complete CDATA stay intact.
/// Callers retain responsibility for the surrounding profile's attributes and state.
pub fn parse_mace_item_element(
    item: roxmltree::Node<'_, '_>,
    data: &GameDataPackage,
) -> Result<ValidatedMaceWeapon, MaceItemError> {
    parse_mace_item(&decode_item_payload(item)?, data)
}

/// Preserve item bytes before XML text normalization; shared by equipment families.
pub(crate) fn decode_item_payload(item: roxmltree::Node<'_, '_>) -> Result<String, MaceItemError> {
    if !item.has_tag_name("Item")
        || item.tag_name().namespace().is_some()
        || item.children().count() != 1
        || !item.first_child().is_some_and(|child| child.is_text())
    {
        return Err(invalid("item must contain one unsplit text payload"));
    }
    let element = &item.document().input_text()[item.range()];
    // Source profile admission restricts Item attributes; nevertheless scan quoted
    // values here so this public helper does not mistake an attribute's '>' for a tag.
    let mut quote = None;
    let mut start = None;
    for (offset, byte) in element.bytes().enumerate() {
        match quote {
            Some(value) if byte == value => quote = None,
            Some(_) => {}
            None if matches!(byte, b'\'' | b'"') => quote = Some(byte),
            None if byte == b'>' => {
                start = Some(offset + 1);
                break;
            }
            _ => {}
        }
    }
    let start = start.ok_or_else(|| invalid("item opening tag missing"))?;
    let end = element
        .rfind("</")
        .ok_or_else(|| invalid("item closing tag missing"))?;
    let content = element
        .get(start..end)
        .ok_or_else(|| invalid("invalid item source range"))?;
    if content.len() > MAX_MACE_ITEM_BYTES * 6 {
        return Err(invalid("encoded item text exceeds bounded payload size"));
    }
    if let Some(cdata) = content
        .strip_prefix("<![CDATA[")
        .and_then(|text| text.strip_suffix("]]>"))
    {
        if cdata.contains("]]>") || cdata.contains("<!--") {
            return Err(invalid(
                "unsupported split or comment-containing CDATA item",
            ));
        }
        return Ok(cdata.into());
    }
    if content.contains('<') {
        return Err(invalid("item contains unsupported XML text fragments"));
    }
    let mut decoded = String::with_capacity(content.len().min(MAX_MACE_ITEM_BYTES));
    let mut rest = content;
    while let Some((prefix, entity)) = rest.split_once('&') {
        decoded.push_str(prefix);
        let Some((name, next)) = entity.split_once(';') else {
            return Err(invalid("unterminated item entity"));
        };
        decoded.push(match name {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ => {
                return Err(invalid(
                    "item supports only named XML entities compatible with PoB",
                ));
            }
        });
        rest = next;
        if decoded.len() > MAX_MACE_ITEM_BYTES {
            return Err(invalid("decoded item text exceeds bounded payload size"));
        }
    }
    decoded.push_str(rest);
    if decoded.len() > MAX_MACE_ITEM_BYTES {
        return Err(invalid("decoded item text exceeds bounded payload size"));
    }
    Ok(decoded)
}

/// Admit a bounded exact payload with explicit local rolls. Only line-edge ASCII
/// whitespace and empty lines are ignored for interpretation; source bytes are retained.
/// Rarity/name/base, Item Level, Quality, optional LevelReq and Implicits: 0 must appear
/// in that order. Every following line must match exactly one selected rule in full.
pub fn parse_mace_item(
    input: &str,
    data: &GameDataPackage,
) -> Result<ValidatedMaceWeapon, MaceItemError> {
    parse_mace_item_inner(input, data, None)
}

/// Expanded equipment assembly is separate from the historical local-only API.
pub(crate) fn parse_mace_equipment(
    input: &str,
    data: &GameDataPackage,
    item_id: u32,
) -> Result<ValidatedMaceWeapon, MaceItemError> {
    parse_mace_item_inner(input, data, Some(item_id))
}
fn parse_mace_item_inner(
    input: &str,
    data: &GameDataPackage,
    item_id: Option<u32>,
) -> Result<ValidatedMaceWeapon, MaceItemError> {
    if input.is_empty() || input.len() > MAX_MACE_ITEM_BYTES {
        return Err(invalid("item payload must contain 1..=8192 bytes"));
    }
    if input
        .chars()
        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        return Err(invalid(
            "item payload contains unsupported control characters",
        ));
    }
    let mut offset = 0;
    let mut lines = Vec::new();
    for (number, full) in input.split_inclusive('\n').enumerate() {
        let raw = full.strip_suffix('\n').unwrap_or(full);
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        if raw.len() > MAX_LINE_BYTES {
            return Err(invalid("item line exceeds 256 bytes"));
        }
        let trimmed = raw.trim_ascii();
        if !trimmed.is_empty() {
            lines.push(SourceLine {
                number: number + 1,
                range: offset..offset + raw.len(),
                raw,
                trimmed,
            });
        }
        offset += full.len();
    }
    if lines.len() > MAX_MACE_MODIFIER_LINES + 7 {
        return Err(invalid(
            "item exceeds 64 local modifier lines plus metadata",
        ));
    }
    let rarity = match lines.first().map(|line| line.trimmed) {
        Some("Rarity: NORMAL") => MaceItemRarity::Normal,
        Some("Rarity: RARE") => MaceItemRarity::Rare,
        _ => return Err(invalid("only exact NORMAL or RARE rarity is supported")),
    };
    let mut at = 1;
    let rare_name = if rarity == MaceItemRarity::Rare {
        let name = lines
            .get(at)
            .ok_or_else(|| invalid("rare item name missing"))?
            .trimmed;
        if name.len() > 128
            || !name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b' ' | b'\'' | b'-'))
        {
            return Err(invalid("rare item name contains unsupported syntax"));
        }
        at += 1;
        Some(name.to_owned())
    } else {
        None
    };
    let base_name = lines
        .get(at)
        .ok_or_else(|| invalid("item base missing"))?
        .trimmed;
    let weapon = data
        .weapons
        .iter()
        .find(|base| base.name == base_name)
        .ok_or_else(|| invalid("item base is not supplied by the selected data package"))?;
    at += 1;
    let item_level = unsigned(header(lines.get(at), "Item Level: ")?, 1, 100, "item level")?;
    at += 1;
    let quality = unsigned(header(lines.get(at), "Quality: ")?, 0, 20, "weapon quality")?;
    at += 1;
    let explicit_level_requirement = if let Some(value) = lines
        .get(at)
        .and_then(|line| line.trimmed.strip_prefix("LevelReq: "))
    {
        at += 1;
        Some(unsigned(value, 0, 100, "equip level")?)
    } else {
        None
    };
    if lines.get(at).map(|line| line.trimmed) != Some("Implicits: 0") {
        return Err(invalid(
            "exact Implicits: 0 is required; runes, enchants and implicit lines are unsupported",
        ));
    }
    at += 1;
    if lines.len() - at > MAX_MACE_MODIFIER_LINES {
        return Err(invalid(
            "at most 64 explicit local modifier lines are supported",
        ));
    }
    let mut modifier_lines = Vec::new();
    let mut local_modifiers = Vec::new();
    let mut actor_modifiers = Vec::new();
    let actor_source = item_id.map(|id| {
        format!(
            "Item:{id}:{}",
            rare_name
                .as_ref()
                .map(|name| format!("{name}, {base_name}"))
                .unwrap_or_else(|| base_name.into())
        )
    });
    for line in &lines[at..] {
        let mut found = None;
        for rule in &data.item_modifier_rules {
            if let Some(values) = match_rule(line.trimmed, rule)? {
                if found.is_some() {
                    return Err(invalid(format!(
                        "line {} matches multiple configured modifier rules",
                        line.number
                    )));
                }
                found = Some(ItemModifierRoll {
                    rule_id: rule.id.clone(),
                    values,
                });
            }
        }
        let actor = actor_source
            .as_ref()
            .map(|source| {
                crate::actor_modifiers::match_equipment_modifier_line(line.trimmed, source, data)
                    .map_err(|e| invalid(e.to_string()))
            })
            .transpose()?
            .flatten();
        if found.is_some() && actor.is_some() {
            return Err(invalid(format!(
                "line {} ambiguously matches local and actor grammar",
                line.number
            )));
        }
        let (rule_id, values, effective_values) = if let Some(mut roll) = found {
            let values = roll.values.clone();
            let rule = data
                .item_modifier_rules
                .iter()
                .find(|rule| rule.id == roll.rule_id)
                .expect("matched local rule");
            roll.values = crate::item_formatting::effective_values(
                line.trimmed,
                &values,
                data,
                &rule
                    .captures
                    .iter()
                    .map(|kind| matches!(kind, ItemCaptureKind::UnsignedInteger))
                    .collect::<Vec<_>>(),
            )
            .map_err(invalid)?;
            if roll.values.len() == 2 && roll.values[0] > roll.values[1] {
                return Err(invalid(
                    "formatted local flat damage minimum exceeds maximum",
                ));
            }
            let evidence = (roll.rule_id.clone(), values, roll.values.clone());
            local_modifiers.push(roll);
            evidence
        } else if let Some(actor) = actor {
            // Item:BuildModListForSlotNum adds a hand condition to untagged Accuracy.
            // A tagged attribute condition is retained unchanged by that source branch.
            if actor
                .records()
                .iter()
                .any(|r| r.stat == ActorStat::Accuracy && r.tags.is_empty())
            {
                return Err(invalid(
                    "weapon Accuracy requires the downstream hand-specific condition",
                ));
            }
            actor_modifiers.extend_from_slice(actor.records());
            (
                actor.rule_id().into(),
                actor.values().to_vec(),
                actor.effective_values().to_vec(),
            )
        } else {
            return Err(invalid(format!(
                "line {} is unknown, malformed or outside the supported local modifier grammar",
                line.number
            )));
        };
        modifier_lines.push(MaceModifierLine {
            line_number: line.number,
            byte_range: line.range.clone(),
            source: line.raw.into(),
            rule_id,
            values,
            effective_values,
        });
    }
    Ok(ValidatedMaceWeapon {
        weapon_key: weapon.id.clone(),
        base_name: base_name.into(),
        rarity,
        rare_name,
        quality,
        item_level,
        explicit_level_requirement,
        effective_level_requirement: explicit_level_requirement
            .unwrap_or(weapon.requirements.level),
        source_text: input.into(),
        source_sha256: format!("{:x}", Sha256::digest(input.as_bytes())),
        local_modifiers,
        modifier_lines,
        actor_modifiers,
    })
}

#[cfg(test)]
#[path = "mace_item_tests.rs"]
mod tests;
