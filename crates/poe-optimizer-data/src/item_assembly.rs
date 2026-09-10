//! Source-owned common item assembly policy. Catalog presence never grants execution.
use crate::{
    game_data::{ActorData, ActorNumericOperation, GameDataError},
    item_loading::{ItemLoadingCatalog, ItemLoadingData, ItemLoadingSource, ItemRuneLoadingPolicy},
    item_scalability::{ItemScalabilityCatalog, ItemScalabilityData},
    modifier_parser::{ModifierParserCatalog, ModifierParserData, ParserTable},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

type Result<T> = std::result::Result<T, GameDataError>;
fn error(message: impl std::fmt::Display) -> GameDataError {
    GameDataError(format!("item assembly policy: {message}"))
}

pub const ITEM_ASSEMBLY_SCHEMA_VERSION: u32 = 1;
pub const ITEM_ASSEMBLY_SOURCE_ROLES: &[&str] = &[
    "add_mod",
    "and64",
    "and_pair",
    "apply_range",
    "build_mod_list",
    "build_slot",
    "build_slots",
    "calc_local",
    "copy_table",
    "create_mod",
    "flag_internal",
    "flag_query",
    "independent_variants",
    "keyword_match",
    "list_internal",
    "list_query",
    "mod_list_constructor",
    "mod_store_constructor",
    "new_mod",
    "not64",
    "primary_slot",
    "ranged_mods",
    "round",
    "round_symmetric",
    "rune_display",
    "scale_add_mod",
    "set_source",
    "variant_check",
    "variant_count",
    "variant_groups",
    "zero_line",
];

#[derive(Debug, Clone, Copy, Eq, PartialOrd, Ord, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ItemAssemblyCapability {
    PolicyOnly,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyData {
    pub schema_version: u32,
    pub capability: ItemAssemblyCapability,
    pub source: ItemLoadingSource,
    pub policy: ItemAssemblyPolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyPolicy {
    pub kinds: ItemAssemblyModKinds,
    pub collection: ItemAssemblyCollectionPolicy,
    pub range: ItemAssemblyRangePolicy,
    pub local: ItemAssemblyLocalPolicy,
    pub nil_queries: ItemAssemblyNilQueryPolicy,
    pub rune: ItemAssemblyRunePolicy,
    pub grants: ItemAssemblyGrantPolicy,
    pub jewel_restrictions: ItemAssemblyJewelRestrictionPolicy,
    pub named_compatibility: Vec<ItemAssemblyNamedRule>,
    pub requirements: ItemAssemblyRequirementPolicy,
    pub slots: ItemAssemblySlotPolicy,
    pub scale: ItemAssemblyScalePolicy,
    pub jewel_item_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyModKinds {
    pub base: String,
    pub increased: String,
    pub more: String,
    pub flag: String,
    pub list: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialOrd, Ord, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ItemAssemblyRows {
    Enchant,
    Rune,
    ClassRequirement,
    Implicit,
    Explicit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyCollectionPolicy {
    pub rows: [ItemAssemblyRows; 5],
    pub source_prefix: String,
    pub source_separator: String,
    pub missing_item_id: f64,
    pub duplicate_alternate_count: u16,
    pub class_find_pattern: String,
    pub class_variant_rewrite: ItemAssemblyTextRewrite,
    pub class_capture_pattern: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyTextRewrite {
    pub pattern: String,
    pub replacement: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyRangePolicy {
    pub range_find_pattern: String,
    pub newline_rewrite: ItemAssemblyTextRewrite,
}

/// An actual calcLocal call, not a global numerical effect declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyLocalQuery {
    pub name: String,
    pub mod_type: String,
    pub flags: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyLocalPolicy {
    pub keyword_flags: f64,
    pub in_slot_tag_type: String,
    pub more_offset: f64,
    pub more_divisor: f64,
}

/// Only original single-name nil-config List/Flag queries in this phase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyNilQueryPolicy {
    pub flags: f64,
    pub keyword_flags: f64,
    pub match_all_field: String,
    pub captured_match_all_mask: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyRunePolicy {
    pub bonded_unlock: ItemAssemblyLocalQuery,
    pub effect_flags: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyGrantPolicy {
    pub query_name: String,
    pub unknown_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyJewelRestrictionPolicy {
    pub query_name: String,
    pub entries: Vec<ItemAssemblyJewelRestriction>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyJewelRestriction {
    pub base_name: String,
    pub query: ItemAssemblyLocalQuery,
}

/// Both original independent if-statements remain independent ordered rules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyNamedRule {
    pub names: Vec<String>,
    pub modifier: ItemAssemblyKeyValueMod,
    #[serde(deserialize_with = "required_option")]
    pub requirement_override: Option<ItemAssemblyRequirementOverride>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyKeyValueMod {
    pub name: String,
    pub mod_type: String,
    pub key: String,
    pub value: f64,
}

#[derive(Debug, Clone, Copy, Eq, PartialOrd, Ord, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ItemAssemblyAttribute {
    Strength,
    Dexterity,
    Intelligence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyRequirementOverride {
    pub attribute: ItemAssemblyAttribute,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyRequirementPolicy {
    pub no_attributes: ItemAssemblyLocalQuery,
    pub converted: ItemAssemblyLocalQuery,
    pub attributes: [ItemAssemblyAttributeRule; 3],
    pub conversion_read_order: [ItemAssemblyAttribute; 3],
    pub converted_write_order: [ItemAssemblyAttribute; 3],
    pub ordinary_write_order: [ItemAssemblyAttribute; 3],
    pub percent_divisor: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyAttributeRule {
    pub attribute: ItemAssemblyAttribute,
    pub conversion: ItemAssemblyLocalQuery,
    pub base: ItemAssemblyLocalQuery,
    pub increased: ItemAssemblyLocalQuery,
    pub source_addends: [ItemAssemblyAttribute; 2],
    pub conversion_subtrahends: [ItemAssemblyAttribute; 2],
}

/// Closed, nonrecursive predicates used only by original slot selection.
/// A Vec of them is an ordered short-circuit OR, not arbitrary control flow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ItemAssemblySlotPredicate {
    BaseWeaponTruthy,
    BaseTypeEquals { value: String },
    ItemTypeEquals { value: String },
    BaseSubtypeEquals { value: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyPrimarySlotRule {
    pub any: Vec<ItemAssemblySlotPredicate>,
    pub slot_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblySlotCountOverride {
    pub item_type: String,
    pub count: u16,
}

#[derive(Debug, Clone, Copy, Eq, PartialOrd, Ord, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ItemAssemblyTagReplacementRole {
    SlotName,
    Hand,
    OtherSlotNumber,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyTagReplacement {
    pub role: ItemAssemblyTagReplacementRole,
    pub pattern: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblySlotPolicy {
    pub primary: Vec<ItemAssemblyPrimarySlotRule>,
    pub multislot_any: Vec<ItemAssemblySlotPredicate>,
    pub default_multislot_count: u16,
    pub slot_count_overrides: Vec<ItemAssemblySlotCountOverride>,
    pub renamed_slot: u16,
    pub slot_name_rewrite: ItemAssemblyTextRewrite,
    pub primary_hand_slot: u16,
    pub primary_hand_name: String,
    pub other_hand_name: String,
    pub other_number_for_primary: String,
    pub other_number_for_other: String,
    pub slot_number_tag_type: String,
    pub tag_replacements: [ItemAssemblyTagReplacement; 3],
    pub crafted_quality: ItemAssemblyLocalQuery,
    pub quality_name_prefix: String,
    pub quality_mod_type: String,
    pub quality_source: String,
    pub spirit_base: ItemAssemblyLocalQuery,
    pub spirit_increased: ItemAssemblyLocalQuery,
    pub spirit_percent_divisor: f64,
    pub charm_limit: ItemAssemblyLocalQuery,
    pub socketed_jewel_effect: ItemAssemblyLocalQuery,
    pub socketed_jewel_percent_divisor: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyScalePolicy {
    pub integer_scaled_key: String,
    pub keyed_value_decimal_places: u8,
    pub truncation_round_decimal_places: u8,
}

#[derive(Debug, Clone)]
pub struct ItemAssemblyCatalog(Arc<ItemAssemblyData>);
impl ItemAssemblyCatalog {
    pub fn new(data: ItemAssemblyData) -> Result<Self> {
        data.validate()?;
        Ok(Self(Arc::new(data)))
    }
    pub fn data(&self) -> &ItemAssemblyData {
        &self.0
    }
    pub fn policy(&self) -> &ItemAssemblyPolicy {
        &self.0.policy
    }
    pub fn source(&self) -> &ItemLoadingSource {
        &self.0.source
    }
    pub fn bind<'a>(
        &'a self,
        items: &'a ItemLoadingCatalog,
        scalability: &'a ItemScalabilityCatalog,
        actor: &'a ActorData,
        parser: &'a ModifierParserCatalog,
    ) -> Result<ItemAssemblyDefinitions<'a>> {
        self.data()
            .validate_dependencies(items.data(), scalability.data(), parser.data())?;
        crate::actor::validate_high_precision_mods(&actor.high_precision_mods)?;
        Ok(ItemAssemblyDefinitions {
            assembly: self,
            items,
            scalability,
            actor,
            parser,
        })
    }
}
#[derive(Debug, Clone, Copy)]
pub struct ItemAssemblyDefinitions<'a> {
    assembly: &'a ItemAssemblyCatalog,
    items: &'a ItemLoadingCatalog,
    scalability: &'a ItemScalabilityCatalog,
    actor: &'a ActorData,
    parser: &'a ModifierParserCatalog,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemAssemblyRuneEffect {
    Global,
    Rune,
    SoulCore,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemAssemblyName<'a> {
    Text(&'a str),
    Parts([&'a str; 3]),
}
impl<'a> ItemAssemblyName<'a> {
    pub fn bytes(self) -> impl Iterator<Item = u8> + 'a {
        let parts = match self {
            Self::Text(text) => [text, "", ""],
            Self::Parts(parts) => parts,
        };
        parts.into_iter().flat_map(str::bytes)
    }
    pub fn equals(self, other: &[u8]) -> bool {
        self.bytes().eq(other.iter().copied())
    }
}
#[derive(Debug, Clone, Copy)]
pub struct ItemAssemblyBorrowedQuery<'a> {
    pub name: ItemAssemblyName<'a>,
    pub mod_type: &'a str,
    pub flags: f64,
}
impl<'a> ItemAssemblyDefinitions<'a> {
    pub fn policy(&self) -> &'a ItemAssemblyPolicy {
        self.assembly.policy()
    }
    pub fn items(&self) -> &'a ItemLoadingCatalog {
        self.items
    }
    pub fn scalability(&self) -> &'a ItemScalabilityCatalog {
        self.scalability
    }
    pub fn rune_policy(&self) -> &'a ItemRuneLoadingPolicy {
        &self.items.policy().rune_loading
    }
    pub fn keyword_flags(&self) -> &'a ParserTable {
        self.parser
            .table(self.parser.data().policy.keyword_flags)
            .expect("validated parser keyword table")
    }
    pub fn rune_effect_query(&self, role: ItemAssemblyRuneEffect) -> ItemAssemblyBorrowedQuery<'a> {
        let p = self.rune_policy();
        let name = match role {
            ItemAssemblyRuneEffect::Global => ItemAssemblyName::Text(&p.effect_global_name),
            ItemAssemblyRuneEffect::Rune => ItemAssemblyName::Parts([
                &p.effect_name_prefix,
                &p.rune_augment_type,
                &p.effect_name_suffix,
            ]),
            ItemAssemblyRuneEffect::SoulCore => ItemAssemblyName::Parts([
                &p.effect_name_prefix,
                &p.extra_slot_augment_type,
                &p.effect_name_suffix,
            ]),
        };
        ItemAssemblyBorrowedQuery {
            name,
            mod_type: &p.effect_mod_type,
            flags: self.policy().rune.effect_flags,
        }
    }
    pub fn high_precision_mods(&self) -> &'a BTreeMap<String, BTreeMap<ActorNumericOperation, u8>> {
        &self.actor.high_precision_mods
    }
}

fn digest(s: &str, len: usize) -> bool {
    s.len() == len
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn path(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with('/')
        && !s.contains(['\\', ':', '\0'])
        && !s.split('/').any(|p| p.is_empty() || p == "." || p == "..")
}
struct Budget {
    bytes: usize,
}
impl Budget {
    fn text(&mut self, s: &str) -> Result<()> {
        if s.len() > 4096 {
            return Err(error("text length bound"));
        }
        self.bytes = self
            .bytes
            .checked_add(s.len())
            .ok_or_else(|| error("text byte overflow"))?;
        if self.bytes > 256 * 1024 {
            return Err(error("aggregate policy/source text bound"));
        }
        Ok(())
    }
    fn number(&self, n: f64) -> Result<()> {
        if n.is_finite() {
            Ok(())
        } else {
            Err(error("nonfinite policy literal"))
        }
    }
    fn count(&self, n: usize, max: usize) -> Result<()> {
        if n <= max {
            Ok(())
        } else {
            Err(error("policy count bound"))
        }
    }
    fn rewrite(&mut self, r: &ItemAssemblyTextRewrite) -> Result<()> {
        self.text(&r.pattern)?;
        self.text(&r.replacement)
    }
    fn query(&mut self, q: &ItemAssemblyLocalQuery) -> Result<()> {
        self.text(&q.name)?;
        self.text(&q.mod_type)?;
        self.number(q.flags)
    }
    fn predicates(&mut self, predicates: &[ItemAssemblySlotPredicate]) -> Result<()> {
        self.count(predicates.len(), 64)?;
        if predicates.is_empty() {
            return Err(error("empty slot predicate disjunction"));
        }
        for p in predicates {
            match p {
                ItemAssemblySlotPredicate::BaseWeaponTruthy => {}
                ItemAssemblySlotPredicate::BaseTypeEquals { value }
                | ItemAssemblySlotPredicate::ItemTypeEquals { value }
                | ItemAssemblySlotPredicate::BaseSubtypeEquals { value } => self.text(value)?,
            }
        }
        Ok(())
    }
}
fn unique<T: Ord + Copy>(values: impl IntoIterator<Item = T>, count: usize) -> Result<()> {
    if values.into_iter().collect::<BTreeSet<_>>().len() == count {
        Ok(())
    } else {
        Err(error("duplicate or missing closed role"))
    }
}
impl ItemAssemblyData {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != ITEM_ASSEMBLY_SCHEMA_VERSION
            || !digest(&self.source.upstream_revision, 40)
        {
            return Err(error("schema or source revision"));
        }
        let mut b = Budget { bytes: 0 };
        b.count(self.source.files.len(), 512)?;
        b.count(self.source.construction_spans.len(), 64)?;
        b.count(self.source.module_order.len(), 512)?;
        if !self
            .source
            .construction_spans
            .keys()
            .map(String::as_str)
            .eq(ITEM_ASSEMBLY_SOURCE_ROLES.iter().copied())
        {
            return Err(error("missing or unknown complete source role"));
        }
        if self.source.files.is_empty() || self.source.construction_spans.is_empty() {
            return Err(error("missing source evidence"));
        }
        b.text(&self.source.upstream_revision)?;
        for (p, h) in &self.source.files {
            if !path(p) || !digest(h, 64) {
                return Err(error("source path/hash"));
            }
            b.text(p)?;
            b.text(h)?;
        }
        for (role, s) in &self.source.construction_spans {
            if role.is_empty()
                || role.contains('\0')
                || !self.source.files.contains_key(&s.path)
                || s.line == 0
                || s.end_line < s.line
                || s.end_line > 1_000_000
                || !digest(&s.sha256, 64)
            {
                return Err(error("source span"));
            }
            b.text(role)?;
            b.text(&s.path)?;
            b.text(&s.sha256)?;
        }
        for p in &self.source.module_order {
            if !self.source.files.contains_key(p) {
                return Err(error("unknown source module"));
            }
            b.text(p)?;
        }
        let p = &self.policy;
        for value in [
            &p.kinds.base,
            &p.kinds.increased,
            &p.kinds.more,
            &p.kinds.flag,
            &p.kinds.list,
            &p.jewel_item_type,
        ] {
            b.text(value)?;
        }
        let c = &p.collection;
        unique(c.rows, 5)?;
        b.count(c.duplicate_alternate_count.into(), 256)?;
        for value in [
            &c.source_prefix,
            &c.source_separator,
            &c.class_find_pattern,
            &c.class_capture_pattern,
        ] {
            b.text(value)?;
        }
        b.number(c.missing_item_id)?;
        b.rewrite(&c.class_variant_rewrite)?;
        b.text(&p.range.range_find_pattern)?;
        b.rewrite(&p.range.newline_rewrite)?;
        b.number(p.local.keyword_flags)?;
        b.text(&p.local.in_slot_tag_type)?;
        b.number(p.local.more_offset)?;
        b.number(p.local.more_divisor)?;
        b.number(p.nil_queries.flags)?;
        b.number(p.nil_queries.keyword_flags)?;
        b.text(&p.nil_queries.match_all_field)?;
        b.number(p.nil_queries.captured_match_all_mask)?;
        b.query(&p.rune.bonded_unlock)?;
        b.number(p.rune.effect_flags)?;
        b.text(&p.grants.query_name)?;
        b.text(&p.grants.unknown_name)?;
        b.text(&p.jewel_restrictions.query_name)?;
        b.count(p.jewel_restrictions.entries.len(), 256)?;
        for r in &p.jewel_restrictions.entries {
            b.text(&r.base_name)?;
            b.query(&r.query)?;
        }
        b.count(p.named_compatibility.len(), 256)?;
        for r in &p.named_compatibility {
            b.count(r.names.len(), 256)?;
            if r.names.is_empty() {
                return Err(error("empty named compatibility predicate"));
            }
            for name in &r.names {
                b.text(name)?;
            }
            for value in [&r.modifier.name, &r.modifier.mod_type, &r.modifier.key] {
                b.text(value)?;
            }
            b.number(r.modifier.value)?;
            if let Some(v) = &r.requirement_override {
                b.number(v.value)?;
            }
        }
        let r = &p.requirements;
        b.query(&r.no_attributes)?;
        b.query(&r.converted)?;
        b.number(r.percent_divisor)?;
        unique(r.attributes.iter().map(|r| r.attribute), 3)?;
        for order in [
            r.conversion_read_order,
            r.converted_write_order,
            r.ordinary_write_order,
        ] {
            unique(order, 3)?;
        }
        for a in &r.attributes {
            b.query(&a.conversion)?;
            b.query(&a.base)?;
            b.query(&a.increased)?;
            for order in [a.source_addends, a.conversion_subtrahends] {
                unique(order, 2)?;
                if order.contains(&a.attribute) {
                    return Err(error("attribute operands must be other attributes"));
                }
            }
        }
        let s = &p.slots;
        b.count(s.primary.len(), 256)?;
        for r in &s.primary {
            b.predicates(&r.any)?;
            b.text(&r.slot_name)?;
        }
        b.predicates(&s.multislot_any)?;
        b.count(s.default_multislot_count.into(), 256)?;
        b.count(s.slot_count_overrides.len(), 256)?;
        for r in &s.slot_count_overrides {
            b.text(&r.item_type)?;
            b.count(r.count.into(), 256)?;
        }
        b.count(s.renamed_slot.into(), 256)?;
        b.count(s.primary_hand_slot.into(), 256)?;
        b.rewrite(&s.slot_name_rewrite)?;
        for v in [
            &s.primary_hand_name,
            &s.other_hand_name,
            &s.other_number_for_primary,
            &s.other_number_for_other,
            &s.slot_number_tag_type,
            &s.quality_name_prefix,
            &s.quality_mod_type,
            &s.quality_source,
        ] {
            b.text(v)?;
        }
        unique(s.tag_replacements.iter().map(|r| r.role), 3)?;
        for r in &s.tag_replacements {
            b.text(&r.pattern)?;
        }
        for q in [
            &s.crafted_quality,
            &s.spirit_base,
            &s.spirit_increased,
            &s.charm_limit,
            &s.socketed_jewel_effect,
        ] {
            b.query(q)?;
        }
        b.number(s.spirit_percent_divisor)?;
        b.number(s.socketed_jewel_percent_divisor)?;
        b.text(&p.scale.integer_scaled_key)?;
        b.count(p.scale.keyed_value_decimal_places.into(), 15)?;
        b.count(p.scale.truncation_round_decimal_places.into(), 15)?;
        Ok(())
    }
    pub fn validate_dependencies(
        &self,
        items: &ItemLoadingData,
        scalability: &ItemScalabilityData,
        parser: &ModifierParserData,
    ) -> Result<()> {
        for source in [&items.source, &scalability.source, &parser.source] {
            if source.upstream_revision != self.source.upstream_revision {
                return Err(error("dependency source revision mismatch"));
            }
            for (path, hash) in &self.source.files {
                if source.files.get(path).is_some_and(|other| other != hash) {
                    return Err(error("dependency source file mismatch"));
                }
            }
        }
        Ok(())
    }
}

fn required_option<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> std::result::Result<Option<T>, D::Error> {
    Option::<T>::deserialize(d)
}
