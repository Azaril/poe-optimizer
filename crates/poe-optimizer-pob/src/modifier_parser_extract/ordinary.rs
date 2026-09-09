//! Source-authenticated ordinary factory invocation protocols and injected precheck.
use super::*;

const PREFIX: &str = r#"local preFlag, preFlagCap
preFlag, line, preFlagCap = scan(line, preFlagList)
if type(preFlag) == "function" then
 preFlag = preFlag(unpack(preFlagCap))
end"#;
const BRIDGE: &str = r#"local skillTag
skillTag, line = scan(line, preSkillNameList)
local modForm, formCap
modForm, line, formCap = scan(line, formList)
if not modForm then
 return nil, line
end"#;
const FIRST: &str = r#"local modTag, modTag2, tagCap
modTag, line, tagCap = scan(line, modTagList)
if type(modTag) == "function" then
 if tagCap[1]:match("__TAG_PATTERN__") then
  modTag = modTag(tonumber(tagCap[1]), unpack(tagCap))
 else
  modTag = modTag(tagCap[1], unpack(tagCap))
 end
end"#;
const SECOND: &str = r#"if modTag then
 modTag2, line, tagCap = scan(line, modTagList)
 if type(modTag2) == "function" then
  if tagCap[1]:match("__TAG_PATTERN__") then
   modTag2 = modTag2(tonumber(tagCap[1]), unpack(tagCap))
  else
   modTag2 = modTag2(tagCap[1], unpack(tagCap))
  end
 end
end"#;

fn unique_part<'a>(text: &'a str, start: &str, end: &str) -> Result<&'a str> {
    if text.matches(start).count() != 1 || text.matches(end).count() != 1 {
        return Err(error("ordinary factory invocation anchors are not unique"));
    }
    section(text, start, end)
}
fn protocol(lua: &Lua, actual: &str, expected: &str) -> Result<Option<String>> {
    let actual = tokens(actual)?;
    let expected = tokens(expected)?;
    if actual.len() != expected.len() {
        return Err(error("ordinary factory invocation protocol changed"));
    }
    let mut pattern = None;
    for (a, e) in actual.iter().zip(&expected) {
        if e.quoted && e.text == r#""__TAG_PATTERN__""# {
            if !a.quoted || pattern.is_some() {
                return Err(error("tag precheck requires one source string literal"));
            }
            pattern = Some(lua.load(format!("return {}", a.text)).eval::<String>()?);
        } else if a.text != e.text || a.quoted != e.quoted {
            return Err(error("ordinary factory invocation protocol changed"));
        }
    }
    Ok(pattern)
}
pub(super) fn extract(
    lua: &Lua,
    sources: &BTreeMap<String, String>,
    spans: &mut BTreeMap<String, ItemSourceSpan>,
) -> Result<String> {
    let text = source(sources, PARSER)?;
    let prefix = unique_part(
        text,
        "local preFlag, preFlagCap",
        "\n\t-- Check for skill name at the start of the line",
    )?;
    let bridge = unique_part(
        text,
        "local skillTag\n",
        "\n\t-- Check for tags (per-charge, conditionals)",
    )?;
    let first = unique_part(text, "local modTag, modTag2, tagCap", "\n\tif modTag then")?;
    let second = unique_part(
        text,
        "\tif modTag then",
        "\n\t-- Scan for modifier name and skill name",
    )?;
    if !(prefix.as_ptr() < bridge.as_ptr()
        && bridge.as_ptr() < first.as_ptr()
        && first.as_ptr() < second.as_ptr())
    {
        return Err(error("ordinary factory stages were reordered"));
    }
    if protocol(lua, prefix, PREFIX)?.is_some() || protocol(lua, bridge, BRIDGE)?.is_some() {
        return Err(error("unexpected prefix/form precheck pattern"));
    }
    let pattern =
        protocol(lua, first, FIRST)?.ok_or_else(|| error("missing first tag precheck pattern"))?;
    if protocol(lua, second, SECOND)?.as_ref() != Some(&pattern) {
        return Err(error("first and second tag precheck patterns differ"));
    }
    for (name, part) in [
        ("prefix_factory_invocation", prefix),
        ("first_tag_factory_invocation", first),
        ("second_tag_factory_invocation", second),
    ] {
        spans.insert(name.into(), part_span(sources, PARSER, part)?);
    }
    Ok(pattern)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sources() -> BTreeMap<String, String> {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../vendor/path-of-building-poe2")
            .join(PARSER);
        BTreeMap::from([(
            PARSER.into(),
            std::fs::read_to_string(path).unwrap().replace("\r\n", "\n"),
        )])
    }
    #[test]
    fn authentic_ordinary_protocols_extract_equal_pattern_and_complete_spans() {
        let source = sources();
        let mut spans = BTreeMap::new();
        assert_eq!(extract(&Lua::new(), &source, &mut spans).unwrap(), "%d+");
        assert_eq!(spans.len(), 3);
        for span in spans.values() {
            assert_eq!(
                *span,
                line_span(&source, PARSER, span.line as usize, span.end_line as usize).unwrap()
            );
            let body = source[PARSER]
                .lines()
                .skip(span.line as usize - 1)
                .take((span.end_line - span.line + 1) as usize)
                .collect::<Vec<_>>();
            assert_eq!(body.last().unwrap().trim(), "end");
        }
    }
    #[test]
    fn ordinary_precheck_pattern_is_source_derived_including_empty_and_malformed() {
        let source = sources();
        for pattern in ["", "[", "^%d+$", "()"] {
            let changed = source[PARSER].replace(
                r#"tagCap[1]:match("%d+")"#,
                &format!("tagCap[1]:match({pattern:?})"),
            );
            let changed = BTreeMap::from([(PARSER.into(), changed)]);
            assert_eq!(
                extract(&Lua::new(), &changed, &mut BTreeMap::new()).unwrap(),
                pattern
            );
        }
        let changed = source[PARSER].replacen(
            r#"tagCap[1]:match("%d+")"#,
            r#"tagCap[1]:match("^%d+$")"#,
            1,
        );
        assert!(
            extract(
                &Lua::new(),
                &BTreeMap::from([(PARSER.into(), changed)]),
                &mut BTreeMap::new()
            )
            .is_err()
        );
    }
    #[test]
    fn altered_argument_packing_precheck_and_source_gates_reject() {
        let source = sources();
        for (from, to) in [
            (
                "preFlag(unpack(preFlagCap))",
                "preFlag(tonumber(preFlagCap[1]), unpack(preFlagCap))",
            ),
            ("tagCap[1]:match", "tagCap[2]:match"),
            (
                "modTag(tonumber(tagCap[1]), unpack(tagCap))",
                "modTag(unpack(tagCap))",
            ),
            (
                "modTag(tagCap[1], unpack(tagCap))",
                "modTag(tonumber(tagCap[1]), unpack(tagCap))",
            ),
            (
                "modTag2(tonumber(tagCap[1]), unpack(tagCap))",
                "modTag2(tonumber(tagCap[1]))",
            ),
            ("if modTag then", "if true then"),
            (
                "if not modForm then\n\t\treturn nil, line",
                "if not modForm then\n\t\treturn {}, line",
            ),
            (
                "preFlag, line, preFlagCap = scan(line, preFlagList)",
                "preFlag, line, preFlagCap = scan(line, modTagList)",
            ),
        ] {
            assert!(source[PARSER].contains(from), "{from}");
            let changed = source[PARSER].replace(from, to);
            assert!(
                extract(
                    &Lua::new(),
                    &BTreeMap::from([(PARSER.into(), changed)]),
                    &mut BTreeMap::new()
                )
                .is_err(),
                "{from}"
            );
        }
    }
}
