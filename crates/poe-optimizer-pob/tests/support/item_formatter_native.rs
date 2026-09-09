//! Test-only source-value transport, without copying formatter calculations.
use super::{oracle::FormatterOracle, runtime};
use mlua::{Table, Value};
use poe_optimizer_data::item_loading::{
    ItemMetadataTable, ItemMetadataValue, ItemOpaqueFunction, ItemSourceSpan,
};
use poe_optimizer_engine::item_tools::{
    FormatError, FormatInput, FormatResult, ItemFormatter, ParserFeedback, RangeInput,
};
use poe_optimizer_import::item_loading::*;
use sha2::{Digest, Sha256};
pub fn metadata(table: Table) -> ItemMetadataTable {
    metadata_at(table, 0)
}
fn metadata_at(table: Table, depth: usize) -> ItemMetadataTable {
    assert!(depth < 24);
    let mut out = ItemMetadataTable::default();
    for row in table.pairs::<Value, Value>() {
        let (key, value) = row.unwrap();
        let value = metadata_value(value, depth + 1);
        match key {
            Value::String(s) => {
                out.fields.insert(s.to_str().unwrap().to_owned(), value);
            }
            Value::Integer(i) => {
                out.indexed.insert(i, value);
            }
            Value::Number(n) if n.fract() == 0.0 && n.abs() <= 9_007_199_254_740_991.0 => {
                out.indexed.insert(n as i64, value);
            }
            other => panic!("unsupported source metadata key {other:?}"),
        }
    }
    out
}
fn metadata_value(value: Value, depth: usize) -> ItemMetadataValue {
    match value {
        Value::Boolean(v) => ItemMetadataValue::Boolean(v),
        Value::Integer(v) => ItemMetadataValue::Number(v as f64),
        Value::Number(v) => {
            assert!(v.is_finite());
            ItemMetadataValue::Number(v)
        }
        Value::String(v) => ItemMetadataValue::Text(v.to_str().unwrap().to_owned()),
        Value::Table(t) => {
            let count = t.clone().pairs::<Value, Value>().count();
            if count > 0
                && count == t.raw_len()
                && t.clone().pairs::<Value, Value>().all(
                    |row| matches!(row.unwrap().0,Value::Integer(n) if n>=1 && n<=count as i64),
                )
            {
                ItemMetadataValue::Array(
                    t.sequence_values::<Value>()
                        .map(|v| metadata_value(v.unwrap(), depth + 1))
                        .collect(),
                )
            } else {
                ItemMetadataValue::Table(metadata_at(t, depth))
            }
        }
        Value::Function(f) => {
            let info = f.info();
            let path = info.source.unwrap().strip_prefix('@').unwrap().to_owned();
            let line = info.line_defined.unwrap() as u32;
            let end_line = info.last_line_defined.unwrap() as u32;
            let source = runtime::verified(&path).unwrap();
            let content = source
                .split_inclusive('\n')
                .skip(line as usize - 1)
                .take((end_line - line + 1) as usize)
                .collect::<String>();
            ItemMetadataValue::Callback(ItemOpaqueFunction {
                callback: ItemSourceSpan {
                    path,
                    line,
                    end_line,
                    sha256: format!("{:x}", Sha256::digest(content.as_bytes())),
                },
            })
        }
        other => panic!("unrepresented source metadata value {other:?}"),
    }
}

pub fn range_value(oracle: &FormatterOracle, range: RangeInput<'_>) -> Value {
    match range {
        RangeInput::Missing => Value::Nil,
        RangeInput::Scalar(v) => Value::Number(v),
        RangeInput::Values(values) => {
            let table = oracle.source.lua.create_table().unwrap();
            for (i, v) in values.iter().enumerate() {
                if let Some(v) = v {
                    table.raw_set(i + 1, *v).unwrap();
                }
            }
            Value::Table(table)
        }
    }
}
pub fn source(oracle: &FormatterOracle, input: FormatInput<'_>) -> Table {
    oracle.observe(
        "range",
        &[
            oracle.text(input.line),
            range_value(oracle, input.range),
            super::optional(input.value_scalar),
            super::optional(input.base_value_scalar),
        ],
    )
}
pub fn compare(formatter: &ItemFormatter<'_>, input: FormatInput<'_>, source: &Table) -> usize {
    let calls = source
        .get::<Table>("calls")
        .unwrap()
        .sequence_values::<Table>()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    let (actual, requested) = match formatter.apply_range(input) {
        Ok(FormatResult::Complete(text)) => (Ok(text), 0),
        Err(error) => (Err(error), 0),
        Ok(FormatResult::NeedsParser(fallback)) => {
            assert_eq!(
                calls.len(),
                1,
                "formatter requested parse {:?}, original calls differ for {input:?}",
                fallback.parser_text()
            );
            let call = &calls[0];
            assert_eq!(
                fallback.parser_text(),
                call.get::<String>("text").unwrap(),
                "exact precision parser input {input:?}"
            );
            assert!(
                !call.get::<bool>("combined").unwrap(),
                "precision query is ordinary parse"
            );
            let modifiers = call.get::<Option<Table>>("modifiers").unwrap().map(|rows| {
                rows.pairs::<Value, Table>()
                    .map(|r| metadata(r.unwrap().1))
                    .collect::<Vec<_>>()
            });
            let extra = call.get::<Option<String>>("extra").unwrap();
            (
                formatter.resume(
                    fallback,
                    ParserFeedback {
                        modifiers: modifiers.as_deref(),
                        extra: extra.as_deref(),
                    },
                ),
                1,
            )
        }
    };
    assert_eq!(
        requested,
        calls.len(),
        "hidden/omitted precision parser calls for {input:?}"
    );
    if source.get::<bool>("ok").unwrap() {
        super::exact_text(actual, source, &format!("{input:?}"));
    } else {
        let message = source.get::<String>("error").unwrap();
        let expected = match input.range {
            RangeInput::Missing => {
                assert!(
                    message.contains("nil value")
                        && message.contains("attempt to perform arithmetic"),
                    "unexpected missing-range source error: {message}"
                );
                FormatError::SourceError("arithmetic on missing range")
            }
            RangeInput::Values(_) => {
                assert!(
                    message.contains("table value")
                        && message.contains("attempt to perform arithmetic"),
                    "unexpected table-range source error: {message}"
                );
                FormatError::SourceError("arithmetic on range table in fallback")
            }
            RangeInput::Scalar(_) => {
                panic!("classify newly observed original error: {message}; {input:?}")
            }
        };
        assert!(
            message.contains("Modules/ItemTools.lua:"),
            "error did not originate in authenticated formatter: {message}"
        );
        assert_eq!(actual, Err(expected), "error class for {input:?}");
    }
    requested
}
pub fn install_entries(
    oracle: &FormatterOracle,
    catalog: &poe_optimizer_data::item_scalability::ItemScalabilityCatalog,
) {
    let values = oracle.source.lua.create_table().unwrap();
    for (key, rows) in &catalog.data().entries {
        let list = oracle.source.lua.create_table().unwrap();
        for (i, row) in rows.iter().enumerate() {
            let fields = oracle.source.lua.create_table().unwrap();
            fields.set("isScalable", row.is_scalable).unwrap();
            if let Some(formats) = &row.formats {
                fields
                    .set(
                        "formats",
                        oracle
                            .source
                            .lua
                            .create_sequence_from(formats.clone())
                            .unwrap(),
                    )
                    .unwrap();
            }
            list.raw_set(i + 1, fields).unwrap();
        }
        values.set(key.as_str(), list).unwrap();
    }
    oracle
        .source
        .lua
        .globals()
        .get::<Table>("data")
        .unwrap()
        .set("modScalability", values)
        .unwrap();
}
pub fn number(value: Value) -> ItemNumber {
    match value {
        Value::Nil => ItemNumber::Nil,
        Value::Integer(n) => ItemNumber::new(n as f64),
        Value::Number(n) => ItemNumber::new(n),
        other => panic!("expected source number, got {other:?}"),
    }
}
pub fn assert_number(a: ItemNumber, b: ItemNumber, label: &str) {
    match (a, b) {
        (ItemNumber::Finite(a), ItemNumber::Finite(b)) => {
            assert_eq!(a.to_bits(), b.to_bits(), "{label}")
        }
        (ItemNumber::NaN, ItemNumber::NaN) => {}
        (a, b) => assert_eq!(a, b, "{label}"),
    }
}

pub fn compare_state(native: &ItemState, source: &Table) {
    for (key, value) in [
        ("raw", native.raw.as_str()),
        ("name", native.name.as_str()),
        ("namePrefix", native.name_prefix.as_str()),
        ("nameSuffix", native.name_suffix.as_str()),
        ("rarity", native.rarity.as_str()),
    ] {
        assert_eq!(source.get::<String>(key).unwrap(), value, "{key}");
    }
    assert_eq!(
        source.get::<Option<String>>("baseName").unwrap(),
        native.base_name,
        "baseName"
    );
    assert_eq!(
        source.get::<bool>("hasBase").unwrap(),
        native.base_present,
        "base presence"
    );
    assert_eq!(
        source.get::<Option<String>>("type").unwrap(),
        native.item_type,
        "item type"
    );
    assert_eq!(
        source.get::<usize>("itemSocketCount").unwrap(),
        native.item_socket_count
    );
    assert_eq!(
        source.get::<usize>("jewelSocketCount").unwrap(),
        native.jewel_socket_count
    );
    assert_eq!(
        source
            .get::<Table>("sockets")
            .unwrap()
            .sequence_values::<Table>()
            .map(|row| row.unwrap().get::<u32>("group").unwrap())
            .collect::<Vec<_>>(),
        native.sockets
    );
    assert_eq!(
        source
            .get::<Table>("runes")
            .unwrap()
            .sequence_values::<String>()
            .map(Result::unwrap)
            .collect::<Vec<_>>(),
        native.runes
    );
    compare_variants(&native.variants, source);
    for key in represented_scalar_fields() {
        let source_key = if key == "affixes_table" {
            "selectedAffixesTableKey"
        } else {
            key.as_str()
        };
        let original = source.get::<Value>(source_key).unwrap();
        assert_eq!(
            native.retained_fields.contains_key(key),
            !matches!(original, Value::Nil),
            "source scalar presence {key}"
        );
    }
    let lines = source
        .get::<Table>("rawLines")
        .unwrap()
        .sequence_values::<String>()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    assert_eq!(lines, native.raw_lines, "rawLines");
    let requirements = source.get::<Table>("requirements").unwrap();
    assert_eq!(
        requirements.clone().pairs::<String, Value>().count(),
        native.requirements.len(),
        "requirement keys"
    );
    for (key, value) in &native.requirements {
        assert_number(*value, number(requirements.get(key.as_str()).unwrap()), key);
    }
    for (key, value) in &native.retained_fields {
        let source_key = if key == "affixes_table" {
            "selectedAffixesTableKey"
        } else {
            key.as_str()
        };
        match value {
            ItemScalar::Number(n) => {
                assert_number(*n, number(source.get(source_key).unwrap()), key)
            }
            ItemScalar::Boolean(b) => assert_eq!(
                source.get::<Option<bool>>(source_key).unwrap(),
                Some(*b),
                "{key}"
            ),
            ItemScalar::Text(s) => assert_eq!(
                source.get::<Option<String>>(source_key).unwrap().as_ref(),
                Some(s),
                "{key}"
            ),
        }
    }
    for (key, lines) in [
        ("buffModLines", &native.buff_mod_lines),
        ("enchantModLines", &native.enchant_mod_lines),
        ("runeModLines", &native.rune_mod_lines),
        (
            "classRequirementModLines",
            &native.class_requirement_mod_lines,
        ),
        ("implicitModLines", &native.implicit_mod_lines),
        ("explicitModLines", &native.explicit_mod_lines),
    ] {
        let rows = source.get::<Table>(key).unwrap();
        assert_eq!(rows.raw_len(), lines.len(), "{key}");
        for (index, line) in lines.iter().enumerate() {
            let row = rows.get::<Table>(index + 1).unwrap();
            assert_eq!(
                row.get::<String>("line").unwrap(),
                line.line,
                "{key}[{}]",
                index + 1
            );
            assert_number(line.range, number(row.get("range").unwrap()), "range");
            assert_eq!(
                row.get::<Option<String>>("extra").unwrap(),
                line.extra,
                "extra"
            );
            assert_number(
                line.corrupted_range,
                number(row.get("corruptedRange").unwrap()),
                "corruptedRange",
            );
            assert_number(
                line.value_scalar,
                number(row.get("valueScalar").unwrap()),
                &format!("valueScalar for {:?}", line.line),
            );
            let modifiers = row
                .get::<Option<Table>>("modList")
                .unwrap()
                .map(|t| {
                    t.sequence_values::<Table>()
                        .map(|v| metadata(v.unwrap()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            assert_eq!(line.modifiers, modifiers, "complete parsed modifiers");
            let tags = row
                .get::<Option<Table>>("modTags")
                .unwrap()
                .map(|t| {
                    t.sequence_values::<String>()
                        .map(Result::unwrap)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            assert_eq!(line.mod_tags, tags, "ordered modTags");
            let flags = line_flags()
                .iter()
                .filter(|name| row.get::<Option<bool>>(name.as_str()).unwrap() == Some(true))
                .cloned()
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(line.flags, flags, "line flags");
            for (key, selection) in [
                ("variantList", &line.selection.variants),
                ("versionList", &line.selection.versions),
                ("variantGroupList", &line.selection.groups),
            ] {
                let original = row.get::<Option<Table>>(key).unwrap().map(|t| {
                    t.pairs::<u32, bool>()
                        .map(|r| {
                            let (k, v) = r.unwrap();
                            assert!(v);
                            k
                        })
                        .collect::<std::collections::BTreeSet<_>>()
                });
                assert_eq!(*selection, original, "{key}");
            }
        }
    }
}

/// A test-only replay of observed assembly outcomes. This does not implement
/// BuildModList: every result was freshly computed by the original source.
fn represented_scalar_fields() -> &'static Vec<String> {
    static FIELDS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    FIELDS.get_or_init(|| {
        let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
        let policy = snapshot.item_loading().policy();
        let mut fields = [
            "id",
            "charmLimit",
            "spiritValue",
            "runicItem",
            "quality",
            "checkSection",
            "advancedCopy",
            "socketedAugmentTypeOverride",
            "title",
            "corruptible",
            "affixes_table",
            "defaultSocketColor",
            "affixLimit",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<std::collections::BTreeSet<_>>();
        if let Some(table) = policy
            .compatibility
            .get("header_assignments")
            .and_then(ItemMetadataValue::as_table)
        {
            for value in table.fields.values() {
                if let Some(field) = value
                    .as_table()
                    .and_then(|t| t.fields.get("field"))
                    .and_then(ItemMetadataValue::as_str)
                    && !field.contains('.')
                {
                    fields.insert(field.into());
                }
            }
        }
        for kind in ["literal_state_flags", "postparse_line_effects"] {
            if let Some(table) = policy
                .compatibility
                .get(kind)
                .and_then(ItemMetadataValue::as_table)
            {
                for value in table.fields.values() {
                    if let Some(t) = value.as_table() {
                        fields.extend(t.fields.keys().cloned());
                    }
                }
            }
        }
        fields.retain(|key| {
            !key.starts_with("variant")
                && !key.starts_with("hasAltVariant")
                && key != "allowDuplicateVariants"
                && key != "selectedVersion"
        });
        fields.into_iter().collect()
    })
}
fn compare_variants(native: &VariantState, source: &Table) {
    let array = |key| {
        source
            .get::<Option<Table>>(key)
            .unwrap()
            .map(|t| {
                t.sequence_values::<String>()
                    .map(Result::unwrap)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    assert_eq!(native.names, array("variantList"), "variant names");
    assert_eq!(native.versions, array("versionList"), "version names");
    assert_eq!(
        native.has_version_list,
        source
            .get::<Option<Table>>("versionList")
            .unwrap()
            .is_some()
    );
    assert_number(
        native.selected.unwrap_or(ItemNumber::Nil),
        number(source.get("variant").unwrap()),
        "selected variant",
    );
    assert_number(
        native.selected_version.unwrap_or(ItemNumber::Nil),
        number(source.get("selectedVersion").unwrap()),
        "selected version",
    );
    assert_eq!(
        native.allow_duplicates,
        source
            .get::<Option<bool>>("allowDuplicateVariants")
            .unwrap()
            .unwrap_or(false)
    );
    for i in 0..5 {
        let suffix = if i == 0 {
            String::new()
        } else {
            (i + 1).to_string()
        };
        assert_eq!(
            native.has_alternate[i],
            source
                .get::<Option<bool>>(format!("hasAltVariant{suffix}"))
                .unwrap()
                .unwrap_or(false)
        );
        assert_number(
            native.alternate[i].unwrap_or(ItemNumber::Nil),
            number(source.get(format!("variantAlt{suffix}")).unwrap()),
            "alternate variant",
        );
    }
    let selections = source
        .get::<Table>("variantGroupSelections")
        .unwrap()
        .pairs::<u32, Value>()
        .map(|row| {
            let (k, v) = row.unwrap();
            (k, number(v))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        native.group_selections, selections,
        "variant group selections"
    );
    let groups = source
        .get::<Table>("variantGroups")
        .unwrap()
        .pairs::<u32, Table>()
        .map(|row| {
            let (group, variants) = row.unwrap();
            let variants = variants
                .pairs::<u32, Table>()
                .map(|row| {
                    let (variant, versions) = row.unwrap();
                    let versions = versions
                        .pairs::<u32, bool>()
                        .map(|row| {
                            let (version, enabled) = row.unwrap();
                            assert!(enabled);
                            version
                        })
                        .collect::<std::collections::BTreeSet<_>>();
                    (variant, versions)
                })
                .collect::<std::collections::BTreeMap<_, _>>();
            (group, variants)
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(native.groups, groups, "variant group eligibility");
}

fn line_flags() -> &'static std::collections::BTreeSet<String> {
    static FLAGS: std::sync::OnceLock<std::collections::BTreeSet<String>> =
        std::sync::OnceLock::new();
    FLAGS.get_or_init(|| {
        poe_optimizer_data::game_data::bundled_snapshot()
            .unwrap()
            .item_loading()
            .policy()
            .line_flags
            .clone()
    })
}

pub struct OriginalParser<'a> {
    pub oracle: &'a FormatterOracle,
    pub calls: Vec<ParseRequest>,
}
impl ItemLoadProvider for OriginalParser<'_> {
    fn parse_modifier(&mut self, request: &ParseRequest) -> DependencyResult<ParseOutcome> {
        self.calls.push(request.clone());
        let (mods, extra): (Option<Table>, Option<String>) = self
            .oracle
            .source
            .lua
            .globals()
            .get::<mlua::Function>("item_loading_parse_dependency")
            .unwrap()
            .call((request.text.as_str(), request.combined))
            .unwrap();
        DependencyResult::Available(ParseOutcome {
            modifiers: mods.map(|m| {
                m.sequence_values::<Table>()
                    .map(|r| metadata(r.unwrap()))
                    .collect()
            }),
            extra,
        })
    }
}
