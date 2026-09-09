//! Complete authenticated item-scaling data and original dispatcher observations.
use crate::game_data::{GameDataExtractionError, error, hash, section};
use mlua::{Function, HookTriggers, Lua, LuaOptions, StdLib, Table, Value, VmState};
use poe_optimizer_data::item_loading::{ItemLoadingSource, ItemSourceSpan};
use poe_optimizer_data::item_scalability::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
type Result<T> = std::result::Result<T, GameDataExtractionError>;
const SCALABILITY: &str = "src/Data/ModScalability.lua";
const TOOLS: &str = "src/Modules/ItemTools.lua";
const DATA: &str = "src/Modules/Data.lua";
const ITEM: &str = "src/Classes/Item.lua";
const COMMON: &str = "src/Modules/Common.lua";
fn source<'a>(sources: &'a BTreeMap<String, String>, path: &str) -> Result<&'a str> {
    sources
        .get(path)
        .map(String::as_str)
        .ok_or_else(|| error(format!("missing scalability source {path}")))
}
fn span(sources: &BTreeMap<String, String>, path: &str, part: &str) -> Result<ItemSourceSpan> {
    let text = source(sources, path)?;
    let at = part.as_ptr() as usize - text.as_ptr() as usize;
    if at > text.len() {
        return Err(error("scalability span is not from source"));
    }
    let line = text[..at].bytes().filter(|b| *b == b'\n').count() as u32 + 1;
    let end_line = line + part.trim_end().bytes().filter(|b| *b == b'\n').count() as u32;
    let bytes = text
        .split_inclusive('\n')
        .skip(line as usize - 1)
        .take((end_line - line + 1) as usize)
        .collect::<String>();
    Ok(ItemSourceSpan {
        path: path.into(),
        line,
        end_line,
        sha256: hash(bytes.as_bytes()),
    })
}
fn chunk<'a>(
    sources: &'a BTreeMap<String, String>,
    path: &str,
    begin: &str,
    end: &str,
) -> Result<&'a str> {
    section(source(sources, path)?, begin, end)
}
fn dense(t: &Table, maximum: usize) -> Result<Vec<Value>> {
    if t.metatable().is_some() {
        return Err(error("scalability array has a metatable"));
    }
    let mut entries = BTreeMap::new();
    for pair in t.clone().pairs::<Value, Value>() {
        let (key, value) = pair?;
        let Value::Integer(key) = key else {
            return Err(error("scalability array has noninteger keys"));
        };
        if key <= 0 || entries.insert(key, value).is_some() || entries.len() > maximum {
            return Err(error("invalid scalability array bounds"));
        }
    }
    if !entries.keys().copied().eq(1..=entries.len() as i64) {
        return Err(error("scalability array is sparse"));
    }
    Ok(entries.into_values().collect())
}
fn strings(t: &Table, maximum: usize) -> Result<Vec<String>> {
    dense(t, maximum)?
        .into_iter()
        .map(|value| match value {
            Value::String(s) => Ok(s.to_str()?.to_owned()),
            _ => Err(error("scalability array requires exact strings")),
        })
        .collect()
}
fn entries(t: Table) -> Result<BTreeMap<String, Vec<ItemScalabilityValue>>> {
    if t.metatable().is_some() {
        return Err(error("scalability root has a metatable"));
    }
    let mut out = BTreeMap::new();
    for pair in t.pairs::<Value, Value>() {
        let (Value::String(key), Value::Table(rows)) = pair? else {
            return Err(error("invalid scalability root entry"));
        };
        let key = key.to_str()?.to_owned();
        let mut values = Vec::new();
        for row in dense(&rows, 64)? {
            let Value::Table(row) = row else {
                return Err(error("scalability capture must be a table"));
            };
            if row.metatable().is_some() {
                return Err(error("scalability capture has a metatable"));
            }
            let mut flag = None;
            let mut formats = None;
            for pair in row.pairs::<Value, Value>() {
                let (Value::String(field), value) = pair? else {
                    return Err(error("invalid scalability field"));
                };
                match (&*field.to_str()?, value) {
                    ("isScalable", Value::Boolean(value)) => flag = Some(value),
                    ("formats", Value::Table(labels)) => formats = Some(strings(&labels, 32)?),
                    _ => return Err(error("unknown or mistyped scalability field")),
                }
            }
            values.push(ItemScalabilityValue {
                is_scalable: flag.ok_or_else(|| error("missing isScalable"))?,
                formats,
            });
        }
        if key.bytes().filter(|b| *b == b'#').count() != values.len()
            || out.insert(key, values).is_some()
            || out.len() > 50_000
        {
            return Err(error("invalid scalability key, arity or count"));
        }
    }
    Ok(out)
}
fn literal(text: &str) -> Result<f64> {
    let value = text.trim().parse::<f64>().map_err(error)?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(error("nonfinite source literal"))
    }
}
fn exact_literal(text: &str, prefix: &str, suffix: &str) -> Result<f64> {
    let matches: Vec<_> = text.match_indices(prefix).collect();
    if matches.len() != 1 {
        return Err(error(format!(
            "missing/ambiguous scalability literal {prefix}"
        )));
    }
    let rest = &text[matches[0].0 + prefix.len()..];
    let end = rest
        .find(suffix)
        .ok_or_else(|| error("missing scalability literal end"))?;
    literal(&rest[..end])
}
fn assignment_labels(dispatch: &str) -> Result<BTreeSet<String>> {
    let mut labels = BTreeSet::new();
    for line in dispatch.lines() {
        let code = line.split("--").next().unwrap().trim();
        if code.is_empty()
            || code == "if scalability.formats then"
            || code == "for _, format in ipairs(scalability.formats) do"
            || code == "end"
        {
            continue;
        }
        if (code.starts_with("if format == ") || code.starts_with("elseif format == "))
            && code.ends_with(" then")
        {
            let mut conditions = code
                .strip_prefix("elseif ")
                .or_else(|| code.strip_prefix("if "))
                .unwrap()
                .strip_suffix(" then")
                .unwrap();
            loop {
                let rest = conditions
                    .strip_prefix("format == \"")
                    .ok_or_else(|| error("unrepresented format condition"))?;
                let end = rest
                    .find('"')
                    .ok_or_else(|| error("unterminated format label"))?;
                labels.insert(rest[..end].into());
                if rest[end + 1..].is_empty() {
                    break;
                }
                conditions = rest[end + 1..]
                    .strip_prefix(" or ")
                    .ok_or_else(|| error("unrepresented format branch"))?;
            }
        } else if let Some((field, value)) = code.split_once(" = ") {
            match field {
                "precision" | "displayPrecision" => {
                    literal(value)?;
                }
                "ifRequired" if value == "true" || value == "false" => {}
                _ => return Err(error("unrepresented format assignment")),
            }
        } else {
            return Err(error(format!(
                "unrepresented format dispatch statement {code}"
            )));
        }
    }
    Ok(labels)
}
fn number(value: Value) -> Result<Option<f64>> {
    match value {
        Value::Nil => Ok(None),
        Value::Integer(v) => Ok(Some(v as f64)),
        Value::Number(v) if v.is_finite() => Ok(Some(v)),
        _ => Err(error("invalid observed format number")),
    }
}
fn observed_assignments(
    lua: &Lua,
    dispatch: &str,
    labels: &BTreeSet<String>,
) -> Result<BTreeMap<String, ItemFormatAssignments>> {
    let function:Function=lua.load(format!("return function(format, precision, displayPrecision, ifRequired) local scalability={{formats={{format}}}}\n{dispatch}\nreturn precision,displayPrecision,ifRequired end")).eval()?;
    let mut out = BTreeMap::new();
    for label in labels {
        let (p, d, t): (Value, Value, Value) =
            function.call((label.as_str(), Value::Nil, Value::Nil, Value::Nil))?;
        let (ps, ds, ts): (Value, Value, Value) =
            function.call((label.as_str(), 7919.0, 7.0, false))?;
        let p = number(p)?;
        let d = number(d)?;
        let t = match t {
            Value::Nil => None,
            Value::Boolean(v) => Some(v),
            _ => return Err(error("invalid observed ifRequired")),
        };
        if number(ps)? != Some(p.unwrap_or(7919.0))
            || number(ds)? != Some(d.unwrap_or(7.0))
            || ts != Value::Boolean(t.unwrap_or(false))
        {
            return Err(error("format assignments are not constant partial updates"));
        }
        let display_precision = d
            .map(|v| {
                if (0.0..=2.0).contains(&v) && v.fract() == 0.0 {
                    Ok(v as u8)
                } else {
                    Err(error("invalid observed display precision"))
                }
            })
            .transpose()?;
        out.insert(
            label.clone(),
            ItemFormatAssignments {
                precision: p,
                display_precision,
                if_required: t,
            },
        );
    }
    let unknown = "__unrecognized_format_definition__";
    let values: (f64, f64, bool) = function.call((unknown, 7919.0, 7.0, false))?;
    if values != (7919.0, 7.0, false) {
        return Err(error("unknown format no-op behavior changed"));
    }
    Ok(out)
}
pub(crate) fn extract(sources: &BTreeMap<String, String>) -> Result<ItemScalabilityData> {
    let lua = Lua::new_with(
        StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::JIT,
        LuaOptions::default(),
    )?;
    lua.set_memory_limit(128 * 1024 * 1024)?;
    lua.load("jit.off();jit.flush();jit=nil;load=nil;loadstring=nil;loadfile=nil;dofile=nil;collectgarbage=nil;print=nil;package=nil;io=nil;os=nil;debug=nil;ffi=nil;require=nil").exec()?;
    let ticks = Arc::new(AtomicUsize::new(0));
    lua.set_hook(
        HookTriggers::new().every_nth_instruction(10_000),
        move |_, _| {
            if ticks.fetch_add(1, Ordering::Relaxed) > 10_000 {
                Err(mlua::Error::RuntimeError(
                    "scalability extraction instruction bound".into(),
                ))
            } else {
                Ok(VmState::Continue)
            }
        },
    )?;
    let full = source(sources, SCALABILITY)?;
    let values: Table = lua.load(full).set_name(format!("@{SCALABILITY}")).eval()?;
    let entries = entries(values)?;
    let dispatch = chunk(
        sources,
        TOOLS,
        "if scalability.formats then",
        "if scalability.isScalable and",
    )?;
    let labels = assignment_labels(dispatch)?;
    let format_assignments = observed_assignments(&lua, dispatch, &labels)?;
    let data = source(sources, DATA)?;
    let default_high_precision = exact_literal(data, "data.defaultHighPrecision = ", "\n")?;
    if !(0.0..=12.0).contains(&default_high_precision) || default_high_precision.fract() != 0.0 {
        return Err(error("invalid fallback precision"));
    }
    let tools = source(sources, TOOLS)?;
    let missing_range_value = exact_literal(tools, "ranges[rangeIndex] or ", ")")?;
    let antonym_source = chunk(
        sources,
        TOOLS,
        "local antonyms = {",
        "local function antonymFunc",
    )?;
    let antonyms: Table = lua
        .load(format!("{antonym_source}\nreturn antonyms"))
        .eval()?;
    let mut antonym_map = BTreeMap::new();
    for pair in antonyms.pairs::<Value, Value>() {
        let (Value::String(a), Value::String(b)) = pair? else {
            return Err(error("invalid source antonym"));
        };
        antonym_map.insert(a.to_str()?.to_owned(), b.to_str()?.to_owned());
    }
    let catalyst_source = chunk(
        sources,
        ITEM,
        "local catalystList = ",
        "local function normaliseModLine",
    )?;
    let catalyst_function = chunk(
        sources,
        ITEM,
        "local function getCatalystScalar(",
        "local function normaliseModLine",
    )?;
    let default_quality = exact_literal(catalyst_function, "quality = ", "\n")?;
    let formula = catalyst_function
        .split_once("return (")
        .ok_or_else(|| error("missing catalyst percentage expression"))?
        .1;
    let (offset, rest) = formula
        .split_once(" + quality) / ")
        .ok_or_else(|| error("changed catalyst percentage expression"))?;
    let percent_offset = literal(offset)?;
    let percent_divisor = literal(rest.lines().next().unwrap_or(""))?;
    let flag_source = catalyst_function
        .split_once("for _, lineFlag in ipairs(")
        .ok_or_else(|| error("missing catalyst flag tags"))?
        .1
        .split_once(") do")
        .ok_or_else(|| error("invalid catalyst flag tags"))?
        .0;
    let flag_table: Table = lua.load(format!("return {flag_source}")).eval()?;
    let extra_tag_flags = strings(&flag_table, 32)?;
    let (catalyst, source_tags): (Function, Table) = lua
        .load(format!(
            "{catalyst_source}\nreturn getCatalystScalar, catalystTags"
        ))
        .eval()?;
    let first_tags: Table = source_tags.get(1)?;
    let first_tags = strings(&first_tags, 32)?;
    let probe_tag = first_tags
        .first()
        .ok_or_else(|| error("source catalyst has no probe tag"))?;
    let empty = lua.create_table()?;
    let neutral_scalar: f64 = catalyst.call((Value::Nil, empty.clone(), Value::Nil))?;
    let tagged = lua.create_table()?;
    tagged.set("modTags", lua.create_sequence_from([probe_tag.as_str()])?)?;
    for quality in [None, Some(0.0), Some(17.5), Some(-31.0)] {
        let actual: f64 = catalyst.call((1, tagged.clone(), quality))?;
        if actual != (percent_offset + quality.unwrap_or(default_quality)) / percent_divisor {
            return Err(error("source catalyst expression/default mismatch"));
        }
    }
    tagged.set("unscalable", true)?;
    if catalyst.call::<f64>((1, tagged, 17.5))? != neutral_scalar {
        return Err(error("source unscalable catalyst guard mismatch"));
    }
    let mut construction_spans = BTreeMap::new();
    for (key, path, begin, end) in [
        (
            "format_dispatch",
            TOOLS,
            "if scalability.formats then",
            "if scalability.isScalable and",
        ),
        (
            "item_format_value",
            TOOLS,
            "function itemLib.formatValue(",
            "local antonyms = {",
        ),
        (
            "ordered_range_lookup",
            TOOLS,
            "local function checkSubstitutionCombinations(",
            "function itemLib.formatModLine(",
        ),
        (
            "fallback_scaling",
            TOOLS,
            "function itemLib.applyValueScalar(",
            "-- precision is express",
        ),
        (
            "antonyms",
            TOOLS,
            "local antonyms = {",
            "function itemLib.isZeroValueLine(",
        ),
        (
            "high_precision",
            DATA,
            "data.defaultHighPrecision = ",
            "data.weaponTypeInfo = {",
        ),
        (
            "catalysts",
            ITEM,
            "local catalystList = ",
            "local function normaliseModLine",
        ),
        (
            "symmetric_rounding",
            COMMON,
            "function roundSymmetric(",
            "-- Symmetric ceil with precision:",
        ),
    ] {
        let part = chunk(sources, path, begin, end)?;
        construction_spans.insert(key.into(), span(sources, path, part)?);
    }
    construction_spans.insert(
        "scalability_table".into(),
        span(sources, SCALABILITY, full)?,
    );
    let files = [SCALABILITY, TOOLS, DATA, ITEM, COMMON]
        .into_iter()
        .map(|p| Ok((p.into(), hash(source(sources, p)?.as_bytes()))))
        .collect::<Result<_>>()?;
    let result = ItemScalabilityData {
        schema_version: ITEM_SCALABILITY_SCHEMA_VERSION,
        source: ItemLoadingSource {
            upstream_revision: crate::source::UPSTREAM_REVISION.into(),
            files,
            construction_spans,
            module_order: vec![SCALABILITY.into(), TOOLS.into(), ITEM.into()],
        },
        entries,
        format_assignments,
        default_high_precision: default_high_precision as u8,
        missing_range_value,
        antonyms: antonym_map,
        catalyst_scaling: CatalystScalingData {
            default_quality,
            percent_offset,
            percent_divisor,
            neutral_scalar,
            extra_tag_flags,
        },
    };
    result.validate().map_err(error)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sources() -> BTreeMap<String, String> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../vendor/path-of-building-poe2");
        [SCALABILITY, TOOLS, DATA, ITEM, COMMON]
            .into_iter()
            .map(|path| {
                (
                    path.into(),
                    crate::source::read_verified_text(&root, path).unwrap(),
                )
            })
            .collect()
    }
    #[test]
    fn complete_original_scalability_catalog_retains_every_row_and_dispatch_label() {
        let result = extract(&sources()).unwrap();
        assert_eq!(result.entries.len(), 15_090);
        assert_eq!(result.entries.values().map(Vec::len).sum::<usize>(), 12_040);
        assert_eq!(
            result.entries.values().filter(|v| v.is_empty()).count(),
            3_321
        );
        assert_eq!(result.entries.values().map(Vec::len).max(), Some(4));
        assert_eq!(
            result.entries.keys().filter(|k| k.contains('\n')).count(),
            365
        );
        assert_eq!(result.entries.keys().filter(|k| !k.is_ascii()).count(), 1);
        assert_eq!(result.format_assignments.len(), 33);
        let labels: BTreeSet<_> = result
            .entries
            .values()
            .flatten()
            .flat_map(|v| v.formats.iter().flatten())
            .cloned()
            .collect();
        assert_eq!(labels.len(), 35);
        assert_eq!(
            labels
                .difference(&result.format_assignments.keys().cloned().collect())
                .count(),
            14
        );
        assert!(!result.format_assignments.contains_key("negate"));
        assert_eq!(
            result.format_assignments["divide_by_one_hundred_and_negate"].precision,
            Some(100.0)
        );
        assert_eq!(
            result.format_assignments["milliseconds_to_seconds_halved"].precision,
            Some(1000.0)
        );
        assert_eq!(result.default_high_precision, 1);
        assert_eq!(result.missing_range_value, 0.5);
        assert_eq!(result.catalyst_scaling.default_quality, 20.0);
        assert_eq!(result.catalyst_scaling.percent_offset, 100.0);
        assert_eq!(result.catalyst_scaling.percent_divisor, 100.0);
        assert_eq!(result.catalyst_scaling.neutral_scalar, 1.0);
        assert_eq!(
            result.catalyst_scaling.extra_tag_flags,
            ["prefix", "suffix"]
        );
        assert_eq!(result.antonyms.len(), 4);
        let encoded = serde_json::to_vec(&result).unwrap();
        assert_eq!(
            serde_json::from_slice::<ItemScalabilityData>(&encoded).unwrap(),
            result
        );
    }
    #[test]
    fn extraction_rejects_unconsumed_rows_and_unrepresented_dispatch_operations() {
        let lua = Lua::new();
        for text in [
            "return {['#']={{isScalable=true,unknown=true}}}",
            "return {['#']={{isScalable=1}}}",
            "return {['#']={{formats={}}}}",
            "return {['#']={{isScalable=true,formats={[2]='negate'}}}}",
            "return {['#']={{isScalable=true,formats={function()end}}}}",
            "return {['#']={}}",
            "return setmetatable({}, {})",
            "return {['#']={setmetatable({isScalable=true},{})}}",
            "return {['#']={[0]={isScalable=true}}}",
        ] {
            assert!(entries(lua.load(text).eval().unwrap()).is_err(), "{text}");
        }
        let sources = sources();
        let dispatch = chunk(
            &sources,
            TOOLS,
            "if scalability.formats then",
            "if scalability.isScalable and",
        )
        .unwrap();
        for replacement in [
            "precision = nil",
            "precision = precision * 2",
            "unconsumed = 7",
        ] {
            assert!(
                assignment_labels(&dispatch.replacen("precision = 2", replacement, 1)).is_err()
            );
        }
        let mut changed = sources.clone();
        changed
            .get_mut(ITEM)
            .unwrap()
            .push_str("\n-- quality = 40\n");
        // Changes outside the original function do not become a fallback default.
        assert_eq!(
            extract(&changed).unwrap().catalyst_scaling.default_quality,
            20.0
        );
        let mut changed = sources;
        *changed.get_mut(ITEM).unwrap() = changed[ITEM].replacen(
            "return (100 + quality) / 100",
            "return 1 + quality / 100",
            1,
        );
        assert!(extract(&changed).is_err());
    }
    #[test]
    fn original_dispatch_observations_preserve_partial_assignment_order() {
        let sources = sources();
        let lua = Lua::new();
        let dispatch = chunk(
            &sources,
            TOOLS,
            "if scalability.formats then",
            "if scalability.isScalable and",
        )
        .unwrap();
        let labels = assignment_labels(dispatch).unwrap();
        let mapping = observed_assignments(&lua, dispatch, &labels).unwrap();
        assert_eq!(
            mapping["divide_by_three"],
            ItemFormatAssignments {
                precision: Some(3.0),
                display_precision: None,
                if_required: None
            }
        );
        assert_eq!(
            mapping["divide_by_ten_1dp_if_required"],
            ItemFormatAssignments {
                precision: Some(10.0),
                display_precision: Some(1),
                if_required: Some(true)
            }
        );
        let call:Function=lua.load(format!("return function(formats) local scalability={{formats=formats}};local precision;local displayPrecision;local ifRequired;{dispatch};return precision,displayPrecision,ifRequired end")).eval().unwrap();
        let actual: (f64, u8, bool) = call
            .call(
                lua.create_sequence_from([
                    "divide_by_ten_1dp_if_required",
                    "divide_by_three",
                    "negate",
                ])
                .unwrap(),
            )
            .unwrap();
        assert_eq!(actual, (3.0, 1, true));
    }
    #[test]
    fn complete_scalability_package_preserves_previous_sections() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../vendor/path-of-building-poe2");
        let result = crate::game_data::extract_pinned_game_data_for_review(&root).unwrap();
        let bytes = result.package.canonical_bytes().unwrap();
        let loaded = poe_optimizer_data::game_data::GameDataLoader::from_bytes(
            &bytes,
            &poe_optimizer_data::game_data::TrustPolicy::AllowCustom,
            &poe_optimizer_data::game_data::LoadLimits::default(),
        )
        .unwrap();
        let baseline = std::env::var_os("POE_SCALABILITY_BASELINE")
            .map(std::fs::read)
            .transpose()
            .unwrap()
            .unwrap_or_else(|| poe_optimizer_data::game_data::bundled_package_bytes().to_vec());
        let old: serde_json::Value = serde_json::from_slice(&baseline).unwrap();
        let new = serde_json::to_value(loaded.package()).unwrap();
        for (name, value) in old.as_object().unwrap() {
            if name != "manifest" && name != "item_scalability" {
                assert_eq!(value, &new[name], "old section {name} changed");
                assert_eq!(
                    old["manifest"]["section_sha256"][name],
                    new["manifest"]["section_sha256"][name]
                );
            }
        }
        if let Some(out) = std::env::var_os("POE_SCALABILITY_OUTPUT") {
            let out = std::path::PathBuf::from(out);
            std::fs::create_dir_all(&out).unwrap();
            std::fs::write(out.join("game-data.json"), &bytes).unwrap();
            std::fs::write(out.join("game-data.sha256"), format!("{}\n", hash(&bytes))).unwrap();
            std::fs::write(
                out.join("evidence.json"),
                serde_json::to_vec_pretty(&result.evidence).unwrap(),
            )
            .unwrap();
        }
        eprintln!(
            "complete scalability package bytes={} sha256={} sections={} sources={}",
            bytes.len(),
            hash(&bytes),
            result.package.manifest.section_sha256.len(),
            result.evidence.source_files_sha256.len()
        );
    }
}
