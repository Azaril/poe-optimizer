//! Bounded lowering of the authored string migration expressions in ConfigTab.Load.
//! Values come from source literals; unsupported expression shapes fail extraction.
use super::*;

struct Reader<'a> {
    tokens: &'a [Token<'a>],
    at: usize,
    lua: &'a Lua,
}
impl Reader<'_> {
    fn matches(&self, expected: &[&str]) -> bool {
        matches_at(self.tokens, self.at, expected)
    }
    fn take(&mut self, expected: &[&str]) -> Result<()> {
        if !self.matches(expected) {
            return Err(error(format!(
                "unsupported authored-load expression at token {}: expected {}",
                self.at,
                expected.join(" ")
            )));
        }
        self.at += expected.len();
        Ok(())
    }
    fn literal(&mut self) -> Result<String> {
        let token = self
            .tokens
            .get(self.at)
            .ok_or_else(|| error("missing authored-load literal"))?;
        let value = literal(self.lua, token)?;
        self.at += 1;
        Ok(value)
    }
}
fn matches_at(tokens: &[Token<'_>], at: usize, expected: &[&str]) -> bool {
    tokens.get(at..at + expected.len()).is_some_and(|found| {
        found
            .iter()
            .zip(expected)
            .all(|(token, text)| !token.quoted && token.text == *text)
    })
}
fn literal(lua: &Lua, token: &Token<'_>) -> Result<String> {
    if !token.quoted {
        return Err(error("authored-load value is not a source string literal"));
    }
    let value: String = lua.load(format!("return {}", token.text)).eval()?;
    if value.len() > 4096 || value.contains('\0') {
        return Err(error("authored-load source literal exceeds bounds"));
    }
    Ok(value)
}
fn consistent(values: Vec<String>, role: &str) -> Result<String> {
    let Some(first) = values.first() else {
        return Err(error(format!("authored-load {role} is absent")));
    };
    if values.iter().any(|value| value != first) {
        return Err(error(format!(
            "authored-load {role} has distinct source values; expand policy"
        )));
    }
    Ok(first.clone())
}

pub(super) fn extract(
    lua: &Lua,
    sources: &BTreeMap<String, String>,
) -> Result<ConfigAuthoredLoadPolicy> {
    let load_source = method_span(lua, sources, "Load", "GetDefaultState")?;
    let set_active_source = method_span(lua, sources, "SetActiveConfigSet", "")?;
    let text = source(sources, CONFIG_TAB)?;
    let chunk = section(
        text,
        "function ConfigTabClass:Load(",
        "\nfunction ConfigTabClass:GetDefaultState(",
    )?;
    let tokens = tokens(chunk)?;
    if tokens.len() > 20_000 {
        return Err(error("authored-load source token bound"));
    }
    let begin_shape = ["elseif", "node", ".", "attrib", ".", "string", "then"];
    let beginnings: Vec<_> = (0..tokens.len())
        .filter(|at| matches_at(&tokens, *at, &begin_shape))
        .collect();
    // Input and Placeholder each have a string case. Only Input is normalized.
    if beginnings.len() != 2 {
        return Err(error("authored-load string case structure changed"));
    }
    let mut reader = Reader {
        tokens: &tokens,
        at: beginnings[0] + begin_shape.len(),
        lua,
    };
    let mut input_string_rewrites = Vec::new();
    reader.take(&["if"])?;
    loop {
        let first_line = tokens[reader.at - 1].line;
        reader.take(&["node", ".", "attrib", ".", "name", "=", "="])?;
        let key = reader.literal()?;
        reader.take(&[
            "then",
            "self",
            ".",
            "configSets",
            "[",
            "configSetId",
            "]",
            ".",
            "input",
            "[",
            "node",
            ".",
            "attrib",
            ".",
            "name",
            "]",
            "=",
            "node",
            ".",
            "attrib",
            ".",
            "string",
        ])?;
        let mut operations = Vec::new();
        while reader.matches(&[":"]) {
            reader.take(&[":"])?;
            if reader.matches(&["lower", "(", ")"]) {
                reader.take(&["lower", "(", ")"])?;
                operations.push(ConfigStringRewrite::AsciiLower);
            } else {
                reader.take(&["gsub", "("])?;
                let pattern = reader.literal()?;
                reader.take(&[","])?;
                if reader.matches(&["function"]) {
                    if pattern != "(%l)(%w*)" {
                        return Err(error(
                            "authored-load title-case pattern requires a new operation",
                        ));
                    }
                    reader.take(&[
                        "function", "(", "a", ",", "b", ")", "return", "s_upper", "(", "a", ")",
                        ".", ".", "b", "end",
                    ])?;
                    if !super::tokens(text)?.windows(8).any(|w| {
                        matches_at(w, 0, &["local", "s_upper", "=", "string", ".", "upper"])
                    }) {
                        return Err(error("authored-load uppercase capture changed"));
                    }
                    operations.push(ConfigStringRewrite::AsciiTitleWords);
                } else {
                    let replacement = reader.literal()?;
                    operations.push(ConfigStringRewrite::LuaGsub {
                        pattern,
                        replacement,
                    });
                }
                reader.take(&[")"])?;
            }
            if operations.len() > 64 {
                return Err(error("authored-load rewrite operation bound"));
            }
        }
        let last_line = tokens[reader.at - 1].line;
        input_string_rewrites.push(ConfigInputStringRewrite {
            key,
            source: span(
                sources,
                CONFIG_TAB,
                load_source.line as usize + first_line - 1,
                load_source.line as usize + last_line - 1,
            )?,
            operations,
        });
        if input_string_rewrites.len() > 1024 {
            return Err(error("authored-load rewrite count bound"));
        }
        if reader.matches(&["elseif"]) {
            reader.take(&["elseif"])?;
        } else {
            reader.take(&[
                "else",
                "self",
                ".",
                "configSets",
                "[",
                "configSetId",
                "]",
                ".",
                "input",
                "[",
                "node",
                ".",
                "attrib",
                ".",
                "name",
                "]",
                "=",
                "node",
                ".",
                "attrib",
                ".",
                "string",
                "end",
                "elseif",
                "node",
                ".",
                "attrib",
                ".",
                "boolean",
                "then",
            ])?;
            break;
        }
    }
    let mut set_titles = Vec::new();
    let mut block_titles = Vec::new();
    let mut legacy_keys = Vec::new();
    for at in 0..tokens.len() {
        if matches_at(&tokens, at, &["self", ":", "CreateConfigSet", "("]) {
            let mut r = Reader {
                tokens: &tokens,
                at: at + 4,
                lua,
            };
            // The ID is structural; title literals remain source-owned.
            if r.matches(&["1"]) {
                r.take(&["1"])?;
            } else {
                r.take(&["configSetId"])?;
            }
            r.take(&[","])?;
            if r.matches(&["node"]) {
                r.take(&["node", ".", "attrib", ".", "title", "or"])?;
            }
            set_titles.push(r.literal()?);
            r.take(&[")"])?;
        }
        if matches_at(&tokens, at, &["title", "="]) {
            let mut r = Reader {
                tokens: &tokens,
                at: at + 2,
                lua,
            };
            if r.matches(&["node"]) {
                r.take(&["node", ".", "attrib", ".", "title", "or"])?;
            } else if r.matches(&["child"]) {
                r.take(&["child", ".", "attrib", ".", "title", "or"])?;
            }
            block_titles.push(r.literal()?);
        }
        if matches_at(&tokens, at, &["configSet", ".", "input", "."]) {
            let token = tokens
                .get(at + 4)
                .ok_or_else(|| error("missing legacy custom modifier key"))?;
            if token.quoted
                || !token
                    .text
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            {
                return Err(error("unsupported legacy custom modifier key access"));
            }
            legacy_keys.push(token.text.to_owned());
        }
    }
    if set_titles.len() != 3 || block_titles.len() != 4 || legacy_keys.len() != 2 {
        return Err(error(
            "authored-load default or migration structure changed",
        ));
    }
    // Native CreateConfigSet uses this same block policy before any Load call.
    // A differing constructor literal needs a separate policy field, never the
    // arbitrary winner from one of the two methods. Source identity already
    // records the complete CreateConfigSet method independently of Load.
    let constructor = section(
        text,
        "function ConfigTabClass:CreateConfigSet(",
        "\nfunction ConfigTabClass:NewConfigSet(",
    )?;
    let constructor_tokens = super::tokens(constructor)?;
    let mut constructor_titles = Vec::new();
    for at in 0..constructor_tokens.len() {
        if matches_at(&constructor_tokens, at, &["title", "="])
            && constructor_tokens
                .get(at + 2)
                .is_some_and(|token| token.quoted)
        {
            constructor_titles.push(literal(lua, &constructor_tokens[at + 2])?);
        }
    }
    if constructor_titles.len() != 1 {
        return Err(error(
            "configuration constructor block-title structure changed",
        ));
    }
    block_titles.extend(constructor_titles);
    Ok(ConfigAuthoredLoadPolicy {
        source: load_source,
        set_active_source,
        default_set_title: consistent(set_titles, "set title")?,
        default_custom_block_title: consistent(block_titles, "block title")?,
        legacy_custom_mods_key: consistent(legacy_keys, "legacy custom modifier key")?,
        input_string_rewrites,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_authored_policy_preserves_literals_order_and_complete_method_spans() {
        let sources = super::super::tests::sources();
        let lua = Lua::new();
        let policy = extract(&lua, &sources).unwrap();
        assert_eq!(policy.source, span(&sources, CONFIG_TAB, 878, 978).unwrap());
        assert_eq!(
            policy.set_active_source,
            span(&sources, CONFIG_TAB, 1407, 1434).unwrap()
        );
        assert_eq!(policy.default_set_title, "Default");
        assert_eq!(policy.default_custom_block_title, "Default");
        assert_eq!(policy.legacy_custom_mods_key, "customMods");
        assert_eq!(policy.input_string_rewrites.len(), 2);
        assert_eq!(policy.input_string_rewrites[0].key, "enemyIsBoss");
        assert_eq!(
            policy.input_string_rewrites[0].operations,
            vec![
                ConfigStringRewrite::AsciiLower,
                ConfigStringRewrite::AsciiTitleWords,
                ConfigStringRewrite::LuaGsub {
                    pattern: "Uber Atziri".into(),
                    replacement: "Boss".into()
                },
                ConfigStringRewrite::LuaGsub {
                    pattern: "Shaper".into(),
                    replacement: "Pinnacle".into()
                },
                ConfigStringRewrite::LuaGsub {
                    pattern: "Sirus".into(),
                    replacement: "Pinnacle".into()
                },
            ]
        );
        assert_eq!(policy.input_string_rewrites[1].key, "presetBossSkills");
        assert_eq!(
            policy.input_string_rewrites[1].operations,
            vec![ConfigStringRewrite::LuaGsub {
                pattern: "^Uber ".into(),
                replacement: String::new()
            },]
        );
        for rewrite in &policy.input_string_rewrites {
            assert_eq!(
                rewrite.source,
                span(
                    &sources,
                    CONFIG_TAB,
                    rewrite.source.line as usize,
                    rewrite.source.end_line as usize
                )
                .unwrap()
            );
        }
        let mut changed = sources.clone();
        let source = changed.get_mut(CONFIG_TAB).unwrap();
        *source = source
            .replace("\"Default\"", "\"Injected title\"")
            .replace(
                "configSet.input.customMods",
                "configSet.input.injectedLegacy",
            )
            .replace("\"enemyIsBoss\"", "\"injectedInput\"")
            .replace("\"^Uber \"", "\"^Legacy \"");
        let modified = extract(&lua, &changed).unwrap();
        assert_eq!(modified.default_set_title, "Injected title");
        assert_eq!(modified.default_custom_block_title, "Injected title");
        assert_eq!(modified.legacy_custom_mods_key, "injectedLegacy");
        assert_eq!(modified.input_string_rewrites[0].key, "injectedInput");
        assert_eq!(
            modified.input_string_rewrites[1].operations[0],
            ConfigStringRewrite::LuaGsub {
                pattern: "^Legacy ".into(),
                replacement: String::new()
            }
        );
        assert_ne!(modified.source.sha256, policy.source.sha256);
    }
    #[test]
    fn unsupported_authored_expressions_and_inconsistent_defaults_fail_closed() {
        let sources = super::super::tests::sources();
        let lua = Lua::new();
        for (old, new) in [
            (
                "node.attrib.string:lower():gsub",
                "node.attrib.string:upper():gsub",
            ),
            ("(%l)(%w*)", "(%a)(%w*)"),
            (
                "local s_upper = string.upper",
                "local s_upper = string.lower",
            ),
            ("return s_upper(a)..b end", "return s_upper(a)..b..b end"),
            (
                "CreateConfigSet(1, \"Default\")",
                "CreateConfigSet(1, \"Different\")",
            ),
            (
                "configSet.input.customMods = nil",
                "configSet.input.otherKey = nil",
            ),
            (
                "customModsList = { { title = \"Default\", enabled = true, text = \"\" } }",
                "customModsList = { { title = \"Different constructor\", enabled = true, text = \"\" } }",
            ),
        ] {
            let mut changed = sources.clone();
            let text = changed.get_mut(CONFIG_TAB).unwrap();
            assert!(text.contains(old), "test must change actual source: {old}");
            *text = text.replacen(old, new, 1);
            assert!(extract(&lua, &changed).is_err(), "accepted {new}");
        }
    }
}
