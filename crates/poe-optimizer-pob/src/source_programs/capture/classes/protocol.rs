//! Closed Common protocol extraction. Whole bodies and their actual captures
//! are checked before source fields are promoted to native class policy.
use super::*;
use crate::source_programs::tokens::tokens;
const SHAPES: &str = include_str!("source-shapes.json");
const FIELDS: [&str; 9] = [
    "Object",
    "_parentInit",
    "_parent",
    "_object",
    "_className",
    "_parents",
    "_superParents",
    "_unconstructedMeta",
    "_constructorInitialised",
];
pub(super) fn extract(
    graph: &mut Graph<'_>,
    allocation: &Function,
) -> Result<SourceClassConstructionPolicy> {
    let shapes: BTreeMap<String, String> = serde_json::from_str(SHAPES).map_err(error)?;
    let mut fields = BTreeMap::new();
    check(graph, allocation, &shapes["new"], &mut fields)?;
    let captured = upvalues(graph, allocation)?;
    let expected = [
        "s_format",
        "getClass",
        "makeUnconstructedMeta",
        "pairs",
        "parentIndex",
        "parentCall",
        "wrapConstructor",
    ];
    if captured.len() != expected.len() || expected.iter().any(|key| !captured.contains_key(*key)) {
        return Err(error(
            "complete source new protocol has unrepresented captures",
        ));
    }
    for (name, symbol) in [("s_format", "string.format"), ("pairs", "pairs")] {
        primitive(graph, &captured[name].1, symbol)?;
    }
    let helper = |name: &str| -> Result<Function> {
        match &captured[name].1 {
            Value::Function(function) => Ok(function.clone()),
            _ => Err(error(format!("source new helper {name} is not a function"))),
        }
    };
    for name in [
        "getClass",
        "makeUnconstructedMeta",
        "wrapConstructor",
        "parentIndex",
        "parentCall",
    ] {
        check(graph, &helper(name)?, &shapes[name], &mut fields)?;
    }
    for name in ["getClass", "makeUnconstructedMeta", "parentIndex"] {
        if !upvalues(graph, &helper(name)?)?.is_empty() {
            return Err(error(format!(
                "source {name} protocol has unrepresented captures"
            )));
        }
    }
    let wrapper = upvalues(graph, &helper("wrapConstructor")?)?;
    if wrapper.len() != 1 || !wrapper.contains_key("pairs") {
        return Err(error("source wrapper helper has unrepresented captures"));
    }
    primitive(graph, &wrapper["pairs"].1, "pairs")?;
    let parent_call = helper("parentCall")?;
    let parent = upvalues(graph, &parent_call)?;
    if parent.len() != 1 || !parent.contains_key("s_format") {
        return Err(error(
            "source parent-call protocol has unrepresented captures",
        ));
    }
    primitive(graph, &parent["s_format"].1, "string.format")?;
    let parent_index = helper("parentIndex")?;
    let callback = |graph: &mut Graph<'_>, function: Function| -> Result<SourceCallbackId> {
        match graph.value(Value::Function(function), 0)? {
            SourceValue::Callback(id) => Ok(id),
            _ => unreachable!(),
        }
    };
    Ok(SourceClassConstructionPolicy {
        allocation: span(graph, allocation)?,
        parent_call: span(graph, &parent_call)?,
        parent_index: span(graph, &parent_index)?,
        wrap_constructor: span(graph, &helper("wrapConstructor")?)?,
        parent_call_callback: callback(graph, parent_call)?,
        parent_call_format_upvalue: parent["s_format"].0,
        parent_index_callback: callback(graph, parent_index)?,
        object_alias: fields["Object"].clone(),
        parent_init: fields["_parentInit"].clone(),
        proxy_parent: fields["_parent"].clone(),
        proxy_object: fields["_object"].clone(),
        proxy_class_name: fields["_className"].clone(),
        class_name_field: fields["_className"].clone(),
        parent_classes_field: fields["_parents"].clone(),
        super_parents_field: fields["_superParents"].clone(),
        unconstructed_meta_field: fields["_unconstructedMeta"].clone(),
        constructor_initialized_field: fields["_constructorInitialised"].clone(),
    })
}
pub(super) fn upvalues(
    graph: &Graph<'_>,
    function: &Function,
) -> Result<BTreeMap<String, (u16, Value)>> {
    if function.info().what == "C"
        || function
            .environment()
            .is_none_or(|env| env.to_pointer() != graph.observer.globals.to_pointer())
    {
        return Err(error(
            "class protocol helper must be an original-global Lua closure",
        ));
    }
    let mut values = BTreeMap::new();
    for index in 1..=129 {
        let (name, value) = graph.upvalue(function, index)?;
        let Some(name) = name else { return Ok(values) };
        if index > 128 || values.insert(name, (index as u16 - 1, value)).is_some() {
            return Err(error("class helper capture count or duplicate bound"));
        }
    }
    Err(error("class helper capture bound"))
}
fn primitive(graph: &Graph<'_>, value: &Value, symbol: &str) -> Result<()> {
    let Value::Function(function) = value else {
        return Err(error(format!("class primitive {symbol} is not a function")));
    };
    if graph
        .observer
        .opaque_primitives
        .iter()
        .any(|(name, original)| name == symbol && original.to_pointer() == function.to_pointer())
    {
        Ok(())
    } else {
        Err(error(format!(
            "class primitive {symbol} differs from before-source identity"
        )))
    }
}
pub(super) fn span(graph: &Graph<'_>, function: &Function) -> Result<ItemSourceSpan> {
    let info = function.info();
    let name = info
        .source
        .as_deref()
        .and_then(|name| name.strip_prefix('@'))
        .ok_or_else(|| error("class callback lacks a declared source name"))?;
    let path = graph
        .source_names
        .get(name)
        .map(String::as_str)
        .unwrap_or(name);
    let text = graph.sources.get(path).ok_or_else(|| {
        error(format!(
            "class callback source name {name} is not in inventory or explicit aliases"
        ))
    })?;
    let first = info
        .line_defined
        .ok_or_else(|| error("class callback lacks first line"))?;
    let last = info
        .last_line_defined
        .ok_or_else(|| error("class callback lacks last line"))?;
    if first == 0 || last < first || last > text.lines().count() {
        return Err(error("class callback source span is outside inventory"));
    }
    let body = text
        .split_inclusive('\n')
        .skip(first - 1)
        .take(last - first + 1)
        .collect::<String>();
    Ok(ItemSourceSpan {
        path: path.into(),
        line: first.try_into().map_err(error)?,
        end_line: last.try_into().map_err(error)?,
        sha256: hash(body.as_bytes()),
    })
}
fn check(
    graph: &Graph<'_>,
    function: &Function,
    expected: &str,
    fields: &mut BTreeMap<String, String>,
) -> Result<()> {
    let source = span(graph, function)?;
    let text = &graph.sources[&source.path];
    let actual = text
        .split_inclusive('\n')
        .skip(source.line as usize - 1)
        .take((source.end_line - source.line + 1) as usize)
        .collect::<String>();
    if actual.len() > 64 * 1024 {
        return Err(error("complete class protocol function source byte bound"));
    }
    let mut actual = tokens(&actual)?;
    let mut expected = tokens(expected)?;
    // debug spans begin on the function's line, while local declarations may
    // precede it on that same line. The reviewed shape includes this prefix.
    if actual.first().is_some_and(|t| t.text == "local") {
        actual.remove(0);
    }
    if expected.first().is_some_and(|t| t.text == "local") {
        expected.remove(0);
    }
    if actual.len() != expected.len() {
        return Err(error("complete Common class protocol shape changed"));
    }
    for (actual, expected) in actual.iter().zip(expected) {
        let field = if expected.quoted {
            expected
                .text
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
        } else {
            Some(expected.text)
        };
        if let Some(field) = field.filter(|name| FIELDS.contains(name)) {
            let value = if expected.quoted {
                if !actual.quoted {
                    return Err(error("class protocol field literal changed type"));
                }
                actual
                    .text
                    .get(1..actual.text.len() - 1)
                    .ok_or_else(|| error("invalid class field literal"))?
            } else {
                actual.text
            };
            if !crate::source_programs::lowering::identifier(value) || value.len() > 256 {
                return Err(error("class source field identifier is unsupported"));
            }
            if let Some(prior) = fields.insert(field.into(), value.into())
                && prior != value
            {
                return Err(error("class source field identities are inconsistent"));
            }
        } else if actual.text != expected.text || actual.quoted != expected.quoted {
            return Err(error("complete Common class protocol shape changed"));
        }
    }
    Ok(())
}
