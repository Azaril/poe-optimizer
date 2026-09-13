//! Complete pinned normalization, comparator and call-site source shapes.
//! Source literals are read from the authenticated bodies, not copied defaults.
//! Changed literals or control flow require source-pin review before extraction.
use super::*;

const WITNESSES: &[(&str, &str, &str, &str)] = &[
    (
        "stat_ordering_helpers",
        "local function normaliseModLine(line)",
        "---@class Item",
        "3672baf327cc92abeb72d1074a2965e60997bf03037bc5b0c131911b630bda0a",
    ),
    (
        "stat_ordering_apply",
        "\tif self.advancedCopy and (self.rarity == \"UNIQUE\" or self.rarity == \"RELIC\") and not self:UsesVersionedOrGroupedVariants() then",
        "\tif self.advancedCopy or self.crafted then",
        "386d594ba4b226e01fbb4a04f56b3a0a0488fabdf9bb3ae6e69a574db05427ae",
    ),
    (
        "stat_ordering_minimum",
        "local m_min = math.min",
        "local m_max = math.max",
        "d55933311af7e72fe64934221c3c63ae9da0376a277f29094654f20ba18b89dd",
    ),
];

fn after<'a>(body: &'a str, prefix: &str) -> Result<&'a str> {
    body.split_once(prefix)
        .map(|(_, rest)| rest)
        .ok_or_else(|| error(format!("missing stat ordering operand {prefix}")))
}
fn string_literal(input: &str) -> Result<(String, &str)> {
    let input = input.trim_start();
    if !input.starts_with('"') {
        return Err(error("stat ordering operand is not a quoted string"));
    }
    let mut escaped = false;
    for (index, byte) in input.bytes().enumerate().skip(1) {
        if !escaped && byte == b'"' {
            let value = serde_json::from_str(&input[..=index]).map_err(error)?;
            return Ok((value, &input[index + 1..]));
        }
        escaped = !escaped && byte == b'\\';
    }
    Err(error("unterminated stat ordering source string"))
}
fn substitution(input: &str) -> Result<ItemStatOrderingSubstitution> {
    let (pattern, rest) = string_literal(input)?;
    let rest = rest
        .trim_start()
        .strip_prefix(',')
        .ok_or_else(|| error("missing stat ordering substitution separator"))?;
    let (replacement, rest) = string_literal(rest)?;
    if !rest.trim_start().starts_with(')') {
        return Err(error("changed stat ordering substitution arity"));
    }
    Ok(ItemStatOrderingSubstitution {
        pattern,
        replacement,
    })
}
fn identifier(input: &str) -> Result<String> {
    let input = input.trim_start();
    let length = input
        .bytes()
        .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
        .count();
    if length == 0 || length > 128 {
        return Err(error("invalid stat ordering source field"));
    }
    Ok(input[..length].into())
}
fn number(input: &str) -> Result<f64> {
    let input = input.trim_start();
    let length = input
        .bytes()
        .take_while(|b| b.is_ascii_digit() || matches!(*b, b'+' | b'-' | b'.' | b'e' | b'E'))
        .count();
    let value = input[..length].parse::<f64>().map_err(error)?;
    if !value.is_finite() {
        return Err(error("nonfinite stat ordering source operand"));
    }
    Ok(value)
}
fn group(body: &str, side: &str) -> Result<(f64, f64, f64)> {
    let body = after(body, &format!("local {side}Group = "))?
        .lines()
        .next()
        .ok_or_else(|| error("missing stat ordering group expression"))?;
    Ok((
        number(after(
            body,
            &format!("({side}.crafted or {side}.custom) and "),
        )?)?,
        number(after(body, &format!("or {side}.fractured and "))?)?,
        number(
            body.rsplit_once(" or ")
                .ok_or_else(|| error("missing ordinary group"))?
                .1,
        )?,
    ))
}

pub(super) fn extract(
    sources: &BTreeMap<String, String>,
) -> Result<(ItemStatOrderingPolicy, BTreeMap<String, ItemSourceSpan>)> {
    let mut bodies = BTreeMap::new();
    let mut spans = BTreeMap::new();
    for &(name, begin, end, expected) in WITNESSES {
        let (body, source_span) = chunk(sources, ITEM, begin, end)?;
        if body.len() > 32 * 1024 || hash(body.as_bytes()) != expected {
            return Err(error(format!(
                "changed complete stat ordering source {name}"
            )));
        }
        bodies.insert(name, body);
        spans.insert(name.into(), source_span);
    }
    let helpers = bodies["stat_ordering_helpers"];
    let apply = bodies["stat_ordering_apply"];
    let rules = helpers
        .split(":gsub(")
        .skip(1)
        .map(substitution)
        .collect::<Result<Vec<_>>>()?;
    let [normalize_numbers, normalize_ranges, flatten_newlines]: [_; 3] = rules
        .try_into()
        .map_err(|_| error("changed normalization operation count"))?;
    for value in apply.split(":gsub(").skip(1) {
        if substitution(value)? != flatten_newlines {
            return Err(error("divergent exact and normalized newline operations"));
        }
    }
    let unique_rarity = string_literal(after(apply, "self.rarity == ")?)?.0;
    let relic_rarity = string_literal(after(apply, "or self.rarity == ")?)?.0;
    let modifier_table = identifier(after(apply, "pairs(data.itemMods.")?)?;
    let stat_order_field = identifier(after(apply, ", mod.")?)?;
    for value in apply.split(", mod.").skip(1) {
        if identifier(value)? != stat_order_field {
            return Err(error("divergent exact and normalized order fields"));
        }
    }
    let (crafted_custom, fractured, ordinary) = group(helpers, "a")?;
    if group(helpers, "b")? != (crafted_custom, fractured, ordinary) {
        return Err(error("divergent source comparator group operands"));
    }
    let policy = ItemStatOrderingPolicy {
        modifier_table,
        stat_order_field,
        unique_rarity,
        relic_rarity,
        normalize_numbers,
        normalize_ranges,
        flatten_newlines,
        groups: ItemStatOrderingGroups {
            crafted_custom,
            fractured,
            ordinary,
            compare_order_below: number(after(helpers, "elseif aGroup < ")?)?,
        },
    };
    policy.validate().map_err(error)?;
    Ok((policy, spans))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sources() -> BTreeMap<String, String> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../vendor/path-of-building-poe2");
        BTreeMap::from([(
            ITEM.into(),
            crate::source::read_verified_text(&root, ITEM).unwrap(),
        )])
    }
    #[test]
    fn complete_original_ordering_policy_and_provenance_are_acquired_without_running_items() {
        let (policy, spans) = extract(&sources()).unwrap();
        assert_eq!(policy.modifier_table, "Exclusive");
        assert_eq!(policy.stat_order_field, "statOrder");
        assert_eq!(
            (policy.unique_rarity.as_str(), policy.relic_rarity.as_str()),
            ("UNIQUE", "RELIC")
        );
        assert_eq!(
            policy.normalize_numbers,
            ItemStatOrderingSubstitution {
                pattern: "%d+%.?%d*".into(),
                replacement: "#".into(),
            }
        );
        assert_eq!(
            policy.normalize_ranges,
            ItemStatOrderingSubstitution {
                pattern: "%(%-?#%-#%)".into(),
                replacement: "#".into(),
            }
        );
        assert_eq!(
            policy.flatten_newlines,
            ItemStatOrderingSubstitution {
                pattern: "\n".into(),
                replacement: " ".into(),
            }
        );
        assert_eq!(
            policy.groups,
            ItemStatOrderingGroups {
                crafted_custom: 3.0,
                fractured: 1.0,
                ordinary: 2.0,
                compare_order_below: 3.0,
            }
        );
        assert_eq!(
            (
                spans["stat_ordering_helpers"].line,
                spans["stat_ordering_helpers"].end_line
            ),
            (65, 88)
        );
        assert_eq!(
            (
                spans["stat_ordering_apply"].line,
                spans["stat_ordering_apply"].end_line
            ),
            (1605, 1625)
        );
        assert_eq!(spans["stat_ordering_minimum"].line, 9);
    }
    #[test]
    fn changed_literals_control_flow_and_minimum_binding_require_review() {
        let original = sources();
        for (from, to) in [
            ("\"%d+%.?%d*\"", "\"%d+\""),
            ("and 3 or a.fractured", "and 4 or a.fractured"),
            (
                "return sourceOrder[a] < sourceOrder[b]",
                "return sourceOrder[a] > sourceOrder[b]",
            ),
            ("mod.statOrder[index]", "mod.otherOrder[index]"),
            ("local m_min = math.min", "local m_min = math.max"),
        ] {
            assert!(original[ITEM].contains(from));
            let changed = BTreeMap::from([(ITEM.into(), original[ITEM].replace(from, to))]);
            assert!(extract(&changed).is_err(), "accepted changed source {from}");
        }
    }
}
