//! Source-preserving equipment assembly. Local weapon effects are consumed once;
//! only supported surviving global actor records enter the shared actor layer.
use crate::actor_modifiers::{ParsedActorModifierLine, match_actor_modifier_line};
use crate::mace_item::{
    MaceItemRarity, ValidatedMaceWeapon, decode_item_payload, parse_mace_equipment,
};
use poe_optimizer_data::game_data::{
    ActorModifierRecord, EquipmentSlot, GameDataPackage, RequirementData,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::ops::Range;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("unsupported equipment: {0}")]
pub struct EquipmentError(String);
type Result<T> = std::result::Result<T, EquipmentError>;
fn invalid(message: impl Into<String>) -> EquipmentError {
    EquipmentError(message.into())
}

#[derive(Debug, Clone, Serialize)]
pub struct EquipmentModifierLine {
    pub line_number: usize,
    pub byte_range: Range<usize>,
    pub source: String,
    pub implicit: bool,
    pub rule_id: String,
    pub values: Vec<f64>,
    pub records: Vec<ActorModifierRecord>,
}
#[derive(Debug, Clone)]
pub struct ValidatedEquipmentItem {
    pob_item_id: u32,
    base_id: String,
    base_name: String,
    allowed_slots: Vec<String>,
    requirements: RequirementData,
    rarity: MaceItemRarity,
    rare_name: Option<String>,
    item_level: u32,
    quality: u32,
    explicit_level_requirement: Option<u32>,
    source_text: String,
    source_sha256: String,
    weapon: Option<ValidatedMaceWeapon>,
    actor_modifiers: Vec<ActorModifierRecord>,
    modifier_lines: Vec<EquipmentModifierLine>,
}
impl ValidatedEquipmentItem {
    pub fn pob_item_id(&self) -> u32 {
        self.pob_item_id
    }
    pub fn base_id(&self) -> &str {
        &self.base_id
    }
    pub fn base_name(&self) -> &str {
        &self.base_name
    }
    pub fn allowed_slots(&self) -> &[String] {
        &self.allowed_slots
    }
    pub fn requirements(&self) -> &RequirementData {
        &self.requirements
    }
    pub fn rarity(&self) -> MaceItemRarity {
        self.rarity
    }
    pub fn rare_name(&self) -> Option<&str> {
        self.rare_name.as_deref()
    }
    pub fn item_level(&self) -> u32 {
        self.item_level
    }
    pub fn quality(&self) -> u32 {
        self.quality
    }
    pub fn explicit_level_requirement(&self) -> Option<u32> {
        self.explicit_level_requirement
    }
    pub fn source_text(&self) -> &str {
        &self.source_text
    }
    pub fn source_sha256(&self) -> &str {
        &self.source_sha256
    }
    pub fn weapon(&self) -> Option<&ValidatedMaceWeapon> {
        self.weapon.as_ref()
    }
    pub fn actor_modifiers(&self) -> &[ActorModifierRecord] {
        &self.actor_modifiers
    }
    pub fn modifier_lines(&self) -> &[EquipmentModifierLine] {
        &self.modifier_lines
    }
    pub fn diagnostic(&self) -> serde_json::Value {
        serde_json::json!({"schema_version":1,"pob_item_id":self.pob_item_id,
            "base_id":self.base_id,"base_name":self.base_name,"allowed_slots":self.allowed_slots,
            "requirements":self.requirements,"rarity":self.rarity,"rare_name":self.rare_name,
            "item_level":self.item_level,"quality":self.quality,
            "explicit_level_requirement":self.explicit_level_requirement,
            "source_sha256":self.source_sha256,"actor_modifiers":self.actor_modifiers,
            "modifier_lines":self.modifier_lines,"weapon":self.weapon.as_ref().map(|w|w.diagnostic()),
            "affix_legality_verified":false})
    }
    /// Reviewed Item:BuildRaw normalization; modifier spelling and order remain intact.
    pub fn pob_export_lines(&self) -> Vec<String> {
        if let Some(weapon) = &self.weapon {
            return weapon.pob_export_lines();
        }
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
            format!("LevelReq: {}", self.requirements.level),
            "Implicits: 1".into(),
        ]);
        lines.extend(
            self.modifier_lines
                .iter()
                .map(|line| line.source.trim_ascii().to_owned()),
        );
        lines
    }
}
struct Line<'a> {
    number: usize,
    range: Range<usize>,
    raw: &'a str,
    text: &'a str,
}
fn lines(input: &str) -> Result<Vec<Line<'_>>> {
    if input.is_empty() || input.len() > crate::mace_item::MAX_MACE_ITEM_BYTES {
        return Err(invalid("item payload must contain 1..=8192 bytes"));
    }
    if input
        .chars()
        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        return Err(invalid("unsupported control character in item"));
    }
    let mut offset = 0;
    let mut lines = Vec::new();
    for (index, piece) in input.split_inclusive('\n').enumerate() {
        let raw = piece.strip_suffix('\n').unwrap_or(piece);
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        if raw.len() > 256 {
            return Err(invalid("item line exceeds 256 bytes"));
        }
        let text = raw.trim_ascii();
        if !text.is_empty() {
            lines.push(Line {
                number: index + 1,
                range: offset..offset + raw.len(),
                raw,
                text,
            });
        }
        offset += piece.len();
    }
    if lines.len() > 72 {
        return Err(invalid("too many item lines"));
    }
    Ok(lines)
}
fn unsigned(value: &str, min: u32, max: u32, name: &str) -> Result<u32> {
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
        .ok_or_else(|| invalid(format!("{name} outside {min}..={max}")))
}
fn field<'a>(line: Option<&'a Line<'_>>, prefix: &str) -> Result<&'a str> {
    line.and_then(|l| l.text.strip_prefix(prefix))
        .ok_or_else(|| invalid(format!("expected ordered item field {prefix}")))
}
fn evidence(
    line: &Line<'_>,
    parsed: &ParsedActorModifierLine,
    implicit: bool,
) -> EquipmentModifierLine {
    EquipmentModifierLine {
        line_number: line.number,
        byte_range: line.range.clone(),
        source: line.raw.into(),
        implicit,
        rule_id: parsed.rule_id().into(),
        values: parsed.values().to_vec(),
        records: parsed.records().to_vec(),
    }
}
/// Parse a supplied physical item instance against selected injected data. IDs are
/// source identity, independent of equipment slot or identical payloads on other items.
pub fn parse_equipment_item(
    input: &str,
    data: &GameDataPackage,
    pob_item_id: u32,
) -> Result<ValidatedEquipmentItem> {
    if pob_item_id == 0 {
        return Err(invalid("item ID must be positive"));
    }
    let lines = lines(input)?;
    let rarity = match lines.first().map(|l| l.text) {
        Some("Rarity: NORMAL") => MaceItemRarity::Normal,
        Some("Rarity: RARE") => MaceItemRarity::Rare,
        _ => return Err(invalid("only exact NORMAL or RARE rarity is supported")),
    };
    let mut at = 1;
    let rare_name = if rarity == MaceItemRarity::Rare {
        let name = lines
            .get(at)
            .ok_or_else(|| invalid("missing rare item name"))?
            .text;
        if name.len() > 128
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b' ' | b'\'' | b'-'))
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
        .ok_or_else(|| invalid("missing item base"))?
        .text;
    if let Some(base) = data.weapons.iter().find(|base| base.name == base_name) {
        let weapon =
            parse_mace_equipment(input, data, pob_item_id).map_err(|e| invalid(e.to_string()))?;
        let mut requirements = base.requirements;
        requirements.level = weapon.effective_level_requirement();
        return Ok(ValidatedEquipmentItem {
            pob_item_id,
            base_id: base.id.clone(),
            base_name: base_name.into(),
            allowed_slots: vec!["Weapon 1".into()],
            requirements,
            rarity,
            rare_name,
            item_level: weapon.item_level(),
            quality: weapon.quality(),
            explicit_level_requirement: weapon.explicit_level_requirement(),
            source_text: input.into(),
            source_sha256: weapon.source_sha256().into(),
            actor_modifiers: weapon.actor_modifiers().to_vec(),
            modifier_lines: vec![],
            weapon: Some(weapon),
        });
    }
    let base = data
        .jewellery_bases
        .iter()
        .find(|base| base.name == base_name)
        .ok_or_else(|| invalid("item base not supplied by selected equipment data"))?;
    at += 1;
    let item_level = unsigned(field(lines.get(at), "Item Level: ")?, 1, 100, "item level")?;
    at += 1;
    let quality = unsigned(
        field(lines.get(at), "Quality: ")?,
        0,
        0,
        "jewellery quality",
    )?;
    at += 1;
    let explicit_level_requirement = if let Some(value) = lines
        .get(at)
        .and_then(|l| l.text.strip_prefix("LevelReq: "))
    {
        at += 1;
        Some(unsigned(value, 0, 100, "equip level")?)
    } else {
        None
    };
    if lines.get(at).map(|l| l.text) != Some("Implicits: 1") {
        return Err(invalid(
            "selected jewellery requires exactly its one source implicit",
        ));
    }
    at += 1;
    let implicit = lines
        .get(at)
        .ok_or_else(|| invalid("missing jewellery implicit"))?;
    let source_name = rare_name
        .as_ref()
        .map(|name| format!("{name}, {base_name}"))
        .unwrap_or_else(|| base_name.into());
    let source = format!("Item:{pob_item_id}:{source_name}");
    let parsed = match_actor_modifier_line(implicit.text, &source, data)
        .map_err(|e| invalid(e.to_string()))?
        .ok_or_else(|| invalid("unsupported jewellery implicit"))?;
    if parsed.rule_id() != base.implicit.actor_rule_id
        || parsed.values().len() != 1
        || !(base.implicit.minimum..=base.implicit.maximum).contains(&parsed.values()[0])
    {
        return Err(invalid(
            "jewellery implicit does not match selected base rule and source roll range",
        ));
    }
    let mut actor_modifiers = parsed.records().to_vec();
    let mut modifier_lines = vec![evidence(implicit, &parsed, true)];
    at += 1;
    if lines.len() - at > 64 {
        return Err(invalid(
            "at most 64 explicit equipment modifier lines are supported",
        ));
    }
    for line in &lines[at..] {
        let parsed = match_actor_modifier_line(line.text, &source, data)
            .map_err(|e| invalid(e.to_string()))?
            .ok_or_else(|| {
                invalid(format!(
                    "unknown equipment modifier at line {}",
                    line.number
                ))
            })?;
        actor_modifiers.extend_from_slice(parsed.records());
        modifier_lines.push(evidence(line, &parsed, false));
    }
    let mut requirements = base.requirements;
    requirements.level = explicit_level_requirement.unwrap_or(requirements.level);
    let slot = match base.slot {
        EquipmentSlot::Amulet => "Amulet",
    };
    Ok(ValidatedEquipmentItem {
        pob_item_id,
        base_id: base.id.clone(),
        base_name: base_name.into(),
        allowed_slots: vec![slot.into()],
        requirements,
        rarity,
        rare_name,
        item_level,
        quality,
        explicit_level_requirement,
        source_text: input.into(),
        source_sha256: format!("{:x}", Sha256::digest(input.as_bytes())),
        weapon: None,
        actor_modifiers,
        modifier_lines,
    })
}
/// Decode exact Item source before XML newline normalization. Variant, catalyst,
/// enchant, rune and other item metadata require their own supported transformation.
pub fn parse_equipment_item_xml(
    item: roxmltree::Node<'_, '_>,
    data: &GameDataPackage,
) -> Result<ValidatedEquipmentItem> {
    if item.attributes().len() != 1
        || item
            .attributes()
            .any(|a| a.namespace().is_some() || a.name() != "id")
    {
        return Err(invalid(
            "selected Item requires only its exact id attribute",
        ));
    }
    let id = unsigned(
        item.attribute("id")
            .ok_or_else(|| invalid("missing item ID"))?,
        1,
        u32::MAX,
        "item ID",
    )?;
    let text = decode_item_payload(item).map_err(|e| invalid(e.to_string()))?;
    parse_equipment_item(&text, data, id)
}
#[cfg(test)]
#[path = "equipment_tests.rs"]
mod tests;
