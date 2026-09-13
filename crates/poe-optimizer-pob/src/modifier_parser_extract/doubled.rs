//! Closed DOUBLED-form policy: source literals and the complete mutation/allocation split.
use super::*;

const PROTOCOL: &str = r#"elseif modForm == "DOUBLED" then
 local modNameString
 if type(modName) == "table" then
  modNameString = modName[1]
  modName[2] = "__PREFIX__" .. modNameString .. "__SUFFIX__"
 else
  modNameString = modName
  modName = modName and {modName, "__PREFIX__" .. modName .. "__SUFFIX__"}
 end
 if modName then
  modType = { "MORE", "OVERRIDE" }
  modValue = { __MORE__, __OVERRIDE__ }
  modExtraTags = { tag = true }
  modExtraTags[1] = { tag = { type = "Multiplier", var = modNameString .. "__SUFFIX__", globalLimit = __LIMIT__, globalLimitKey = modNameString .. "__LIMIT_SUFFIX__" }}
 end
end"#;

#[derive(Debug, PartialEq)]
pub(super) struct DoubledPolicy {
    pub(super) multiplier_prefix: String,
    pub(super) name_suffix: String,
    pub(super) limit_suffix: String,
    pub(super) more: f64,
    pub(super) override_value: f64,
    pub(super) global_limit: f64,
}

fn literal_number(actual: &[Token<'_>], index: &mut usize) -> Result<f64> {
    let negative = actual
        .get(*index)
        .is_some_and(|t| !t.quoted && t.text == "-");
    if negative {
        *index += 1;
    }
    let token = actual
        .get(*index)
        .ok_or_else(|| error("missing DOUBLED numeric literal"))?;
    if token.quoted
        || !token
            .text
            .as_bytes()
            .first()
            .is_some_and(|c| c.is_ascii_digit() || *c == b'.')
    {
        return Err(error("DOUBLED requires decimal numeric literals"));
    }
    // The shared scanner deliberately emits decimal digits as separate tokens.
    // Rejoin only adjacent source bytes; whitespace cannot turn two literals into one.
    let mut literal = String::new();
    let mut next_byte = token.text.as_ptr() as usize;
    while let Some(token) = actual.get(*index) {
        if !token.quoted && matches!(token.text, "," | "}") {
            break;
        }
        if token.quoted
            || token.text.as_ptr() as usize != next_byte
            || literal.len() + token.text.len() > 128
        {
            return Err(error("DOUBLED requires a bounded decimal numeric literal"));
        }
        literal.push_str(token.text);
        next_byte += token.text.len();
        *index += 1;
    }
    let value = literal.parse::<f64>().map_err(error)?;
    let value = if negative { -value } else { value };
    if !value.is_finite() || value.abs() > 1e12 {
        return Err(error("DOUBLED numeric literal exceeds policy bounds"));
    }
    Ok(value)
}

pub(super) fn extract(
    lua: &Lua,
    sources: &BTreeMap<String, String>,
    spans: &mut BTreeMap<String, ItemSourceSpan>,
) -> Result<DoubledPolicy> {
    let text = source(sources, PARSER)?;
    let start = "elseif modForm == \"DOUBLED\" then";
    let end = "\n\tif not modName then";
    if text.matches(start).count() != 1 || text.matches(end).count() != 1 {
        return Err(error("DOUBLED branch anchors are missing or ambiguous"));
    }
    let part = section(text, start, end)?;
    let actual = tokens(part)?;
    let expected = tokens(PROTOCOL)?;
    let mut index = 0;
    let mut strings = BTreeMap::new();
    let mut numbers = BTreeMap::new();
    for expected in expected {
        if !expected.quoted && matches!(expected.text, "__MORE__" | "__OVERRIDE__" | "__LIMIT__") {
            numbers.insert(expected.text, literal_number(&actual, &mut index)?);
            continue;
        }
        let actual = actual
            .get(index)
            .ok_or_else(|| error("complete DOUBLED branch changed"))?;
        if expected.quoted
            && matches!(
                expected.text,
                "\"__PREFIX__\"" | "\"__SUFFIX__\"" | "\"__LIMIT_SUFFIX__\""
            )
        {
            // Evaluate only an authenticated single string token, never an expression.
            if !actual.quoted || actual.text.len() > 16_384 {
                return Err(error("DOUBLED requires bounded source string literals"));
            }
            let value = lua
                .load(format!("return {}", actual.text))
                .eval::<String>()?;
            if value.len() > 4096 {
                return Err(error("DOUBLED string literal exceeds policy bounds"));
            }
            if let Some(previous) = strings.insert(expected.text, value.clone())
                && previous != value
            {
                return Err(error("DOUBLED repeated text operands differ"));
            }
        } else if actual.text != expected.text || actual.quoted != expected.quoted {
            return Err(error("complete DOUBLED branch changed"));
        }
        index += 1;
    }
    if index != actual.len() {
        return Err(error("complete DOUBLED branch has extra operations"));
    }
    let result = DoubledPolicy {
        multiplier_prefix: strings.remove("\"__PREFIX__\"").unwrap(),
        name_suffix: strings.remove("\"__SUFFIX__\"").unwrap(),
        limit_suffix: strings.remove("\"__LIMIT_SUFFIX__\"").unwrap(),
        more: numbers["__MORE__"],
        override_value: numbers["__OVERRIDE__"],
        global_limit: numbers["__LIMIT__"],
    };
    spans.insert("doubled_form".into(), part_span(sources, PARSER, part)?);
    Ok(result)
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
    fn full_doubled_branch_has_source_operands_and_complete_span() {
        let sources = sources();
        let mut spans = BTreeMap::new();
        assert_eq!(
            extract(&Lua::new(), &sources, &mut spans).unwrap(),
            DoubledPolicy {
                multiplier_prefix: "Multiplier:".into(),
                name_suffix: "Doubled".into(),
                limit_suffix: "DoubledLimit".into(),
                more: 100.0,
                override_value: 1.0,
                global_limit: 100.0,
            }
        );
        assert_eq!(spans.len(), 1);
        assert_eq!(
            spans["doubled_form"],
            line_span(&sources, PARSER, 6900, 6916).unwrap()
        );
    }

    #[test]
    fn doubled_literals_are_injected_with_shared_suffix_agreement() {
        let original = sources();
        for (prefix, suffix, limit) in [
            ("\"\"", "\"x\"", "\"y\""),
            ("\"a\\000\"", "\"long\"", "\"z\""),
        ] {
            let changed = original[PARSER]
                .replace("\"Multiplier:\"", prefix)
                .replace("\"Doubled\"", suffix)
                .replace("\"DoubledLimit\"", limit)
                .replace("modValue = { 100, 1 }", "modValue = { -2.5, 0 }")
                .replace("globalLimit = 100,", "globalLimit = 7.25,");
            let lua = Lua::new();
            let result = extract(
                &lua,
                &BTreeMap::from([(PARSER.into(), changed)]),
                &mut BTreeMap::new(),
            )
            .unwrap();
            assert_eq!(
                result.multiplier_prefix,
                lua.load(format!("return {prefix}"))
                    .eval::<String>()
                    .unwrap()
            );
            assert_eq!(
                result.name_suffix,
                lua.load(format!("return {suffix}"))
                    .eval::<String>()
                    .unwrap()
            );
            assert_eq!(
                result.limit_suffix,
                lua.load(format!("return {limit}"))
                    .eval::<String>()
                    .unwrap()
            );
            assert_eq!(
                (result.more, result.override_value, result.global_limit),
                (-2.5, 0.0, 7.25)
            );
        }
        let changed = original[PARSER].replacen(
            "\"Multiplier:\" .. modNameString .. \"Doubled\"",
            "\"Other:\" .. modNameString .. \"Doubled\"",
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
    fn changed_doubled_mutation_allocation_tags_and_expressions_reject() {
        let original = sources();
        for (from, to) in [
            (
                "if type(modName) == \"table\" then",
                "if type(modName) ~= \"table\" then",
            ),
            (
                "modName[2] = \"Multiplier:\"",
                "modName[3] = \"Multiplier:\"",
            ),
            ("modName = modName and {modName,", "modName = {modName,"),
            ("modExtraTags[1] = { tag =", "modExtraTags[2] = { tag ="),
            (
                "modExtraTags = { tag = true }",
                "modExtraTags = { tag = false }",
            ),
            ("modValue = { 100, 1 }", "modValue = { 100 + 1, 1 }"),
            ("modValue = { 100, 1 }", "modValue = { 1e309, 1 }"),
            (
                "globalLimitKey = modNameString .. \"DoubledLimit\"",
                "globalLimitKey = modNameString .. \"DoubledLimit\", extra = true",
            ),
            (
                "var = modNameString .. \"Doubled\"",
                "var = modNameString .. \"Different\"",
            ),
            ("\"Multiplier:\" .. modName ..", "prefix() .. modName .."),
        ] {
            assert!(original[PARSER].contains(from), "{from}");
            let changed = original[PARSER].replace(from, to);
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
        let changed =
            original[PARSER].replace("\"DoubledLimit\"", &format!("\"{}\"", "x".repeat(4097)));
        assert!(
            extract(
                &Lua::new(),
                &BTreeMap::from([(PARSER.into(), changed)]),
                &mut BTreeMap::new()
            )
            .is_err()
        );
    }
}
