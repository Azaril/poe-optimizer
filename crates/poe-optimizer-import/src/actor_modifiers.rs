//! Strict source-preserving actor modifier admission shared by native profiles and search.
//! Rule wording belongs to injected game data; Rust owns bounded syntax and capability checks.
use poe_optimizer_data::game_data::{
    ActorCaptureKind, ActorModifierEffect, ActorModifierRecord, ActorModifierRule, ActorRuleEffect,
    ActorRuleValue, ActorStat, GameDataPackage,
};
use roxmltree::Node;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::ops::Range;
use thiserror::Error;

pub const MAX_ACTOR_MODIFIER_BYTES: usize = 8 * 1024;
pub const MAX_ACTOR_MODIFIER_LINES: usize = 64;
const MAX_BLOCKS: usize = 16;
// Allow named-entity expansion and block metadata while bounding exact XML evidence.
const MAX_ACTOR_SOURCE_BYTES: usize = MAX_ACTOR_MODIFIER_BYTES * 6 + MAX_BLOCKS * 1024;
const MAX_LINE_BYTES: usize = 256;
const MAX_VALUE: f64 = 1_000_000.0;
#[derive(Debug, Error)]
#[error("unsupported actor modifiers: {0}")]
pub struct ActorModifierError(String);
type Result<T> = std::result::Result<T, ActorModifierError>;
fn invalid(message: impl Into<String>) -> ActorModifierError {
    ActorModifierError(message.into())
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ActorModifierBlock {
    pub title: String,
    pub enabled: bool,
    pub text: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct ActorModifierLine {
    pub block_index: usize,
    pub line_number: usize,
    pub byte_range: Range<usize>,
    pub source: String,
    pub rule_id: String,
    pub values: Vec<f64>,
    pub records: Vec<ActorModifierRecord>,
}
/// Immutable source projection; only the shared parser can construct this evidence.
#[derive(Debug, Clone)]
pub struct ValidatedActorModifiers {
    blocks: Vec<ActorModifierBlock>,
    lines: Vec<ActorModifierLine>,
    records: Vec<ActorModifierRecord>,
    source_fragments: Vec<String>,
    source_format: &'static str,
    authored: bool,
}
impl ValidatedActorModifiers {
    pub fn records(&self) -> &[ActorModifierRecord] {
        &self.records
    }
    pub fn blocks(&self) -> &[ActorModifierBlock] {
        &self.blocks
    }
    pub fn lines(&self) -> &[ActorModifierLine] {
        &self.lines
    }
    /// Enabled authored records requiring the receiving-defence source capability.
    pub fn uses_receiving_defence(&self) -> bool {
        self.records
            .iter()
            .any(|record| record.stat.is_receiving_defence())
    }
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
    /// Includes explicitly authored empty/disabled forms for versioned CLI scope checks.
    pub fn uses_extended_scope(&self) -> bool {
        self.authored
    }
    pub fn diagnostic(&self) -> serde_json::Value {
        serde_json::json!({"schema_version":1,"source_format":self.source_format,
            "source_fragments":self.source_fragments,
            "source_sha256":format!("{:x}",Sha256::digest(serde_json::to_vec(&self.source_fragments).expect("source strings"))),
            "blocks":self.blocks,"lines":self.lines,"records":self.records})
    }
    pub(crate) fn validate_reference_blocks(&self, config_set: Node<'_, '_>) -> Result<()> {
        let nodes: Vec<_> = config_set
            .children()
            .filter(|n| n.has_tag_name("CustomModifierBlock"))
            .collect();
        let expected = if self.blocks.is_empty() {
            vec![ActorModifierBlock {
                title: "Default".into(),
                enabled: true,
                text: String::new(),
            }]
        } else {
            self.blocks.clone()
        };
        if nodes.len() != expected.len() {
            return Err(invalid("reference actor block count changed"));
        }
        for (node, expected) in nodes.into_iter().zip(expected) {
            let actual = parse_block(node)?;
            if node.attribute("title") != Some(expected.title.as_str())
                || node.attribute("enabled")
                    != Some(if expected.enabled { "true" } else { "false" })
                || actual.title != expected.title
                || actual.enabled != expected.enabled
                || actual.text.trim_ascii() != expected.text.trim_ascii()
            {
                return Err(invalid(
                    "reference actor block title, enabled state or text changed",
                ));
            }
        }
        Ok(())
    }
}

/// Parse literal modifier text as one enabled Default block, retaining every source byte.
pub fn parse_actor_modifier_text(
    text: &str,
    data: &GameDataPackage,
) -> Result<ValidatedActorModifiers> {
    prepare(
        vec![ActorModifierBlock {
            title: "Default".into(),
            enabled: true,
            text: text.into(),
        }],
        vec![text.into()],
        "text",
        true,
        data,
    )
}
/// Interpret only actor-bearing children of one already-selected ConfigSet.
/// Encounter Input validation and outer profile shape remain the caller's responsibility.
pub fn parse_actor_configuration(
    config_set: Node<'_, '_>,
    data: &GameDataPackage,
) -> Result<ValidatedActorModifiers> {
    if !config_set.has_tag_name("ConfigSet") || config_set.tag_name().namespace().is_some() {
        return Err(invalid("expected one selected ConfigSet"));
    }
    let mut blocks = Vec::new();
    let mut legacy = None;
    let mut fragments = Vec::new();
    for node in config_set.children().filter(Node::is_element) {
        if node.has_tag_name("CustomModifierBlock") {
            blocks.push(parse_block(node)?);
            push_source_fragment(&mut fragments, node)?;
        } else if node.has_tag_name("Input") && node.attribute("name") == Some("customMods") {
            if node.tag_name().namespace().is_some()
                || node.attributes().len() != 2
                || node
                    .attributes()
                    .any(|a| a.namespace().is_some() || !["name", "string"].contains(&a.name()))
                || node.children().next().is_some()
            {
                return Err(invalid(
                    "legacy customMods must be an empty Input with name/string only",
                ));
            }
            let text = raw_attribute(node, "string")?
                .ok_or_else(|| invalid("customMods requires string"))?;
            if legacy.replace(text).is_some() {
                return Err(invalid("duplicate legacy customMods input"));
            }
            push_source_fragment(&mut fragments, node)?;
        }
    }
    if blocks.len() > MAX_BLOCKS {
        return Err(invalid("too many actor modifier blocks"));
    }
    let authored = !fragments.is_empty();
    let source_format = if let Some(text) = legacy {
        if !blocks.is_empty() {
            return Err(invalid(
                "mixed legacy customMods and modifier blocks are unsupported",
            ));
        }
        blocks.push(ActorModifierBlock {
            title: "Default".into(),
            enabled: true,
            text,
        });
        "legacy_input"
    } else if authored {
        "blocks"
    } else {
        "absent"
    };
    prepare(blocks, fragments, source_format, authored, data)
}
fn push_source_fragment(fragments: &mut Vec<String>, node: Node<'_, '_>) -> Result<()> {
    let source = &node.document().input_text()[node.range()];
    let retained: usize = fragments.iter().map(String::len).sum();
    if source.len() > MAX_ACTOR_SOURCE_BYTES.saturating_sub(retained) {
        return Err(invalid("encoded actor source exceeds 64 KiB"));
    }
    fragments.push(source.to_owned());
    Ok(())
}
fn parse_block(node: Node<'_, '_>) -> Result<ActorModifierBlock> {
    if node.tag_name().namespace().is_some()
        || node
            .attributes()
            .any(|a| a.namespace().is_some() || !["title", "enabled"].contains(&a.name()))
    {
        return Err(invalid("unsupported actor block attributes"));
    }
    let title = raw_attribute(node, "title")?.unwrap_or_else(|| "Default".into());
    if title.len() > 128 || title.chars().any(char::is_control) {
        return Err(invalid(
            "actor block title is unbounded or contains control characters",
        ));
    }
    let enabled = match node.attribute("enabled") {
        None | Some("true") => true,
        Some("false") => false,
        _ => return Err(invalid("actor block enabled must be true or false")),
    };
    Ok(ActorModifierBlock {
        title,
        enabled,
        text: raw_block_text(node)?,
    })
}
fn decode_entities(content: &str) -> Result<String> {
    let mut decoded = String::with_capacity(content.len());
    let mut rest = content;
    while let Some((prefix, entity)) = rest.split_once('&') {
        decoded.push_str(prefix);
        let (name, next) = entity
            .split_once(';')
            .ok_or_else(|| invalid("unterminated XML entity"))?;
        decoded.push(match name {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "apos" => '\'',
            "quot" => '"',
            _ => {
                return Err(invalid(
                    "only PoB-compatible named XML entities are admitted",
                ));
            }
        });
        rest = next;
    }
    decoded.push_str(rest);
    Ok(decoded)
}
fn raw_attribute(node: Node<'_, '_>, name: &str) -> Result<Option<String>> {
    node.attributes()
        .find(|a| a.name() == name)
        .map(|a| decode_entities(&node.document().input_text()[a.range_value()]))
        .transpose()
}
fn raw_block_text(node: Node<'_, '_>) -> Result<String> {
    if node.children().any(|n| !n.is_text()) || node.children().count() > 1 {
        return Err(invalid("actor block must contain one unsplit text payload"));
    }
    let source = &node.document().input_text()[node.range()];
    if source.len() > MAX_ACTOR_MODIFIER_BYTES * 6 + 1024 {
        return Err(invalid("encoded actor block exceeds size limit"));
    }
    let mut quote = None;
    let mut start = None;
    for (offset, byte) in source.bytes().enumerate() {
        match quote {
            Some(value) if value == byte => quote = None,
            Some(_) => {}
            None if matches!(byte, b'\'' | b'"') => quote = Some(byte),
            None if byte == b'>' => {
                start = Some(offset + 1);
                break;
            }
            _ => {}
        }
    }
    let start = start.ok_or_else(|| invalid("missing actor block opening tag"))?;
    if source[..start].ends_with("/>") {
        return Ok(String::new());
    }
    let end = source
        .rfind("</")
        .ok_or_else(|| invalid("missing actor block closing tag"))?;
    let content = source
        .get(start..end)
        .ok_or_else(|| invalid("invalid actor block source range"))?;
    if let Some(text) = content
        .strip_prefix("<![CDATA[")
        .and_then(|v| v.strip_suffix("]]>"))
    {
        if text.contains("]]>") || text.contains("<!--") {
            return Err(invalid("split/comment-containing CDATA is unsupported"));
        }
        return Ok(text.into());
    }
    if content.contains('<') {
        return Err(invalid("unsupported actor block text fragments"));
    }
    decode_entities(content)
}
fn decimal(number: &str) -> bool {
    let mut parts = number.split('.');
    let integral = parts.next().unwrap_or("");
    !integral.is_empty()
        && integral.bytes().all(|b| b.is_ascii_digit())
        && parts
            .next()
            .is_none_or(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
        && parts.next().is_none()
}
fn match_rule(line: &str, rule: &ActorModifierRule) -> Result<Option<Vec<f64>>> {
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
            let Some(parts) = rest.split_once(next) else {
                return Ok(None);
            };
            parts
        };
        let syntax = match kind {
            ActorCaptureKind::SignedDecimal => number.strip_prefix(['+', '-']).is_some_and(decimal),
            ActorCaptureKind::UnsignedInteger => {
                !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit())
            }
            ActorCaptureKind::UnsignedDecimal => decimal(number),
        };
        if !syntax {
            return Ok(None);
        }
        let Some(value) = number
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite() && v.abs() <= MAX_VALUE)
        else {
            return Ok(None);
        };
        values.push(value);
        rest = remaining;
    }
    Ok(rest.is_empty().then_some(values))
}
fn source_capability(record: &ActorModifierRecord) -> Result<()> {
    record.validate().map_err(|e| invalid(e.to_string()))?;
    use ActorStat::*;
    if matches!(
        record.stat,
        ChaosInoculation
            | LifeConvertToEnergyShield
            | LifeConvertToArmour
            | LifeConvertToEvasion
            | ManaConvertToEnergyShield
            | ManaConvertToArmour
            | ManaConvertToEvasion
            | SpiritConvertToEnergyShield
            | SpiritConvertToArmour
            | SpiritConvertToEvasion
    ) {
        return Err(invalid(
            "actor conversion or Chaos Inoculation needs the complete downstream defence stage",
        ));
    }
    if record.flags != 0 || record.keyword_flags != 0 {
        return Err(invalid(
            "actor source modifiers must be global and unscoped",
        ));
    }
    Ok(())
}
/// Exact, source-neutral expansion of one configured actor line. Only this parser
/// constructs the evidence; callers still apply equipment/passive source transformations.
#[derive(Debug, Clone, Serialize)]
pub struct ParsedActorModifierLine {
    rule_id: String,
    values: Vec<f64>,
    records: Vec<ActorModifierRecord>,
}
impl ParsedActorModifierLine {
    pub fn rule_id(&self) -> &str {
        &self.rule_id
    }
    pub fn values(&self) -> &[f64] {
        &self.values
    }
    pub fn records(&self) -> &[ActorModifierRecord] {
        &self.records
    }
}
/// Match an entire literal line without inventing a Custom modifier source. `None`
/// means no rule matched; ambiguous or downstream-incomplete rules are errors.
pub fn match_actor_modifier_line(
    line: &str,
    source: &str,
    data: &GameDataPackage,
) -> Result<Option<ParsedActorModifierLine>> {
    if line.is_empty() || line.len() > MAX_LINE_BYTES || line.chars().any(char::is_control) {
        return Err(invalid("actor line must be bounded literal text"));
    }
    if source.is_empty() || source.len() > 256 || source.chars().any(char::is_control) {
        return Err(invalid("actor source must be bounded literal text"));
    }
    let mut found = None;
    for rule in &data.actor.modifier_rules {
        if let Some(values) = match_rule(line, rule)?
            && found.replace((rule, values)).is_some()
        {
            return Err(invalid("ambiguous actor modifier rule"));
        }
    }
    let Some((rule, values)) = found else {
        return Ok(None);
    };
    let mut records = Vec::new();
    for mapping in &rule.modifiers {
        let effect = match mapping.effect {
            ActorRuleEffect::Flag { value } => ActorModifierEffect::Flag { value },
            ActorRuleEffect::Numeric { operation, value } => {
                let value = match value {
                    ActorRuleValue::Constant { value } => value,
                    ActorRuleValue::Capture { index, multiplier } => {
                        *values
                            .get(index as usize)
                            .ok_or_else(|| invalid("actor rule capture index missing"))?
                            * multiplier
                    }
                };
                if !value.is_finite() || value.abs() > MAX_VALUE {
                    return Err(invalid("actor modifier value exceeds numeric bounds"));
                }
                ActorModifierEffect::Numeric { operation, value }
            }
        };
        let record = ActorModifierRecord {
            stat: mapping.stat,
            effect,
            source: Some(source.into()),
            flags: mapping.flags,
            keyword_flags: mapping.keyword_flags,
            tags: mapping.tags.clone(),
        };
        source_capability(&record)?;
        records.push(record);
    }
    Ok(Some(ParsedActorModifierLine {
        rule_id: rule.id.clone(),
        values,
        records,
    }))
}
fn prepare(
    blocks: Vec<ActorModifierBlock>,
    source_fragments: Vec<String>,
    source_format: &'static str,
    authored: bool,
    data: &GameDataPackage,
) -> Result<ValidatedActorModifiers> {
    let bytes: usize = blocks.iter().map(|b| b.text.len()).sum();
    if bytes > MAX_ACTOR_MODIFIER_BYTES {
        return Err(invalid("actor modifier text exceeds 8 KiB"));
    }
    let mut lines = Vec::new();
    let mut records = Vec::new();
    for (block_index, block) in blocks.iter().enumerate() {
        if block
            .text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\r' | '\n' | '\t'))
        {
            return Err(invalid("unsupported control character in actor text"));
        }
        let mut offset = 0;
        for (line_index, piece) in block.text.split_inclusive('\n').enumerate() {
            let source = piece
                .strip_suffix('\n')
                .unwrap_or(piece)
                .strip_suffix('\r')
                .unwrap_or(piece.strip_suffix('\n').unwrap_or(piece));
            if source.len() > MAX_LINE_BYTES {
                return Err(invalid("actor modifier line exceeds 256 bytes"));
            }
            let text = source.trim_ascii();
            if block.enabled && !text.is_empty() {
                if lines.len() >= MAX_ACTOR_MODIFIER_LINES {
                    return Err(invalid("more than 64 enabled actor modifier lines"));
                }
                let parsed =
                    match_actor_modifier_line(text, &format!("Custom:{}", block.title), data)?
                        .ok_or_else(|| {
                            invalid(format!(
                                "unrecognized actor modifier at block {} line {}: {text}",
                                block_index + 1,
                                line_index + 1
                            ))
                        })?;
                let mapped = parsed.records;
                let values = parsed.values;
                records.extend(mapped.iter().cloned());
                lines.push(ActorModifierLine {
                    block_index,
                    line_number: line_index + 1,
                    byte_range: offset..offset + source.len(),
                    source: source.into(),
                    rule_id: parsed.rule_id,
                    values,
                    records: mapped,
                });
            }
            offset += piece.len();
        }
    }
    Ok(ValidatedActorModifiers {
        blocks,
        lines,
        records,
        source_fragments,
        source_format,
        authored,
    })
}
#[cfg(test)]
#[path = "actor_modifiers_tests.rs"]
mod tests;
