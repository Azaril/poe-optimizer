//! Borrowed source rune definitions. No metadata field is validated before its
//! original operation is reached, and iteration order never claims Lua order.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemRuneTypeRewrite {
    pub item_type: Option<String>,
    pub sub_type: String,
    pub to: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemRuneLoadingPolicy {
    pub rune_header: String,
    pub socket_header: String,
    pub other_headers: BTreeSet<String>,
    pub other_header_patterns: Vec<String>,
    pub socket_character_pattern: String,
    pub item_socket_pattern: String,
    pub jewel_socket_pattern: String,
    pub none_rune_id: String,
    pub rune_table: String,
    pub bonded_skip_pattern: String,
    pub augment_override_pattern: String,
    pub soul_core_pattern: String,
    pub numeric_pattern: String,
    pub stripped_marker: String,
    pub no_number_value: f64,
    pub vector_default: f64,
    pub vector_tolerance: f64,
    pub order_default: f64,
    pub order_separator: String,
    pub bonded_order_marker: String,
    pub bonded_display_prefix: String,
    pub combined_parse_strip_pattern: String,
    pub bonded_range_capture_pattern: String,
    pub bonded_range_strip_pattern: String,
    pub extra_slot_augment_type: String,
    pub rune_augment_type: String,
    pub broad_weapon_type: String,
    pub broad_armour_type: String,
    pub broad_caster_type: String,
    pub caster_tags: Vec<String>,
    pub specific_type_rewrites: Vec<ItemRuneTypeRewrite>,
    pub override_broad_type: String,
    pub game_mode: String,
    pub effect_mod_type: String,
    pub effect_global_name: String,
    pub effect_name_prefix: String,
    pub effect_name_suffix: String,
    pub effect_divisor: f64,
    pub effect_default: f64,
    pub scalar_base: f64,
}
impl ItemRuneLoadingPolicy {
    pub(super) fn validate(&self, policy: &ItemLoadingPolicy) -> Result<()> {
        if self.other_headers.len() > 256
            || self.other_header_patterns.len() > 256
            || self.caster_tags.len() > 256
            || self.specific_type_rewrites.len() > 256
        {
            return Err(error("rune policy count bound"));
        }
        let mut bytes = 0usize;
        let mut check = |s: &str, empty: bool| -> Result<()> {
            text(s, 4096)?;
            if !empty && s.is_empty() {
                return Err(error("empty rune identity or role"));
            }
            bytes = bytes
                .checked_add(s.len())
                .ok_or_else(|| error("rune policy byte overflow"))?;
            if bytes > 128 * 1024 {
                return Err(error("rune policy aggregate byte bound"));
            }
            Ok(())
        };
        for value in [
            &self.rune_header,
            &self.socket_header,
            &self.none_rune_id,
            &self.rune_table,
            &self.extra_slot_augment_type,
            &self.rune_augment_type,
            &self.broad_weapon_type,
            &self.broad_armour_type,
            &self.broad_caster_type,
            &self.override_broad_type,
            &self.game_mode,
            &self.effect_mod_type,
            &self.effect_global_name,
        ] {
            check(value, false)?;
        }
        for value in [
            &self.socket_character_pattern,
            &self.item_socket_pattern,
            &self.jewel_socket_pattern,
            &self.bonded_skip_pattern,
            &self.augment_override_pattern,
            &self.soul_core_pattern,
            &self.numeric_pattern,
            &self.stripped_marker,
            &self.order_separator,
            &self.bonded_order_marker,
            &self.bonded_display_prefix,
            &self.combined_parse_strip_pattern,
            &self.bonded_range_capture_pattern,
            &self.bonded_range_strip_pattern,
            &self.effect_name_prefix,
            &self.effect_name_suffix,
        ] {
            check(value, true)?;
        }
        for value in &self.other_headers {
            check(value, false)?;
        }
        for value in &self.other_header_patterns {
            check(value, true)?;
        }
        for value in &self.caster_tags {
            check(value, false)?;
        }
        for rule in &self.specific_type_rewrites {
            if let Some(v) = &rule.item_type {
                check(v, false)?;
            }
            check(&rule.sub_type, false)?;
            check(&rule.to, false)?;
        }
        if self.rune_header == self.socket_header {
            return Err(error("rune and socket headers collide"));
        }
        for header in [&self.rune_header, &self.socket_header] {
            if !policy.header_names.contains(header)
                || self.other_headers.contains(header)
                || policy.affix_loading.headers.contains_key(header)
                || policy.defence_header_keys.contains_key(header)
                || ["selection_headers", "header_assignments"]
                    .into_iter()
                    .any(|name| {
                        policy
                            .compatibility
                            .get(name)
                            .and_then(ItemMetadataValue::as_table)
                            .is_some_and(|t| t.fields.contains_key(header))
                    })
            {
                return Err(error(
                    "rune header is unknown or conflicts with another operation",
                ));
            }
        }
        if self
            .other_headers
            .iter()
            .any(|h| !policy.header_names.contains(h))
        {
            return Err(error("unknown reserved rune header operation"));
        }
        if self
            .other_header_patterns
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            != self.other_header_patterns.len()
            || self.caster_tags.iter().collect::<BTreeSet<_>>().len() != self.caster_tags.len()
        {
            return Err(error("duplicate rune pattern or caster tag"));
        }
        for value in [
            self.no_number_value,
            self.vector_default,
            self.vector_tolerance,
            self.order_default,
            self.effect_divisor,
            self.effect_default,
            self.scalar_base,
        ] {
            if !value.is_finite() {
                return Err(error("nonfinite rune policy number"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ItemRuneCatalog<'a> {
    table: &'a ItemMetadataTable,
}
impl<'a> ItemRuneCatalog<'a> {
    pub(super) fn new(table: &'a ItemMetadataTable) -> Self {
        Self { table }
    }
    pub fn table(self) -> &'a ItemMetadataTable {
        self.table
    }
    /// Raw presence, including false, scalar, and malformed values. Consumers
    /// apply the original operation's truthiness and type checks when reached.
    pub fn lookup(self, name: &str) -> Option<&'a ItemMetadataValue> {
        self.table.fields.get(name)
    }
    /// Stable storage enumeration, never a source traversal-order guarantee.
    pub fn identities(self) -> impl Iterator<Item = (&'a str, &'a ItemMetadataValue)> {
        self.table.fields.iter().map(|(k, v)| (k.as_str(), v))
    }
}
#[derive(Debug, Clone, Copy)]
pub enum ItemRuneRecord<'a> {
    Table(&'a ItemMetadataTable),
    Array(&'a [ItemMetadataValue]),
}
impl<'a> ItemRuneRecord<'a> {
    pub fn new(value: &'a ItemMetadataValue) -> Option<Self> {
        match value {
            ItemMetadataValue::Table(t) => Some(Self::Table(t)),
            ItemMetadataValue::Array(a) => Some(Self::Array(a)),
            _ => None,
        }
    }
    pub fn field(self, name: &str) -> Option<&'a ItemMetadataValue> {
        match self {
            Self::Table(t) => t.fields.get(name),
            Self::Array(_) => None,
        }
    }
    pub fn indexed(self, index: i64) -> Option<&'a ItemMetadataValue> {
        match self {
            Self::Table(t) => t.indexed.get(&index),
            Self::Array(a) => usize::try_from(index.checked_sub(1)?)
                .ok()
                .and_then(|i| a.get(i)),
        }
    }
    /// Exact ipairs prefix: false is present, a missing index terminates, and
    /// entries beyond the first hole stay available through indexed()/raw data.
    pub fn dense_prefix(self) -> impl Iterator<Item = &'a ItemMetadataValue> {
        (1..=i64::MAX).map_while(move |i| self.indexed(i))
    }
}
