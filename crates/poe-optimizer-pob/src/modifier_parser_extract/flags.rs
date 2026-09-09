//! One authenticated variadic wrapper over the original constructor, not helper execution.
use super::*;

const HELPER: &str = r#"local function flag(name, ...)
 return mod(name, "__TYPE__", __VALUE__, ...)
end"#;
pub(super) fn source_policy(
    lua: &Lua,
    sources: &BTreeMap<String, String>,
    spans: &mut BTreeMap<String, ItemSourceSpan>,
) -> Result<(String, bool)> {
    let text = source(sources, PARSER)?;
    let start = "local function flag(";
    if text.matches(start).count() != 1 {
        return Err(error("flag declaration is missing or ambiguous"));
    }
    let part = section(text, start, "\nend")?;
    let at = part.as_ptr() as usize - text.as_ptr() as usize;
    let part = &text[at..at + part.len() + 4];
    let actual = tokens(part)?;
    let expected = tokens(HELPER)?;
    if actual.len() != expected.len() {
        return Err(error("complete flag helper changed"));
    }
    let mut kind = None;
    let mut value = None;
    for (a, e) in actual.iter().zip(&expected) {
        if e.quoted && e.text == r#""__TYPE__""# {
            if !a.quoted {
                return Err(error("flag requires a literal string constructor type"));
            }
            kind = Some(lua.load(format!("return {}", a.text)).eval::<String>()?);
        } else if e.text == "__VALUE__" {
            value = Some(match a.text {
                "true" if !a.quoted => true,
                "false" if !a.quoted => false,
                _ => return Err(error("flag requires a literal boolean constructor value")),
            });
        } else if a.text != e.text || a.quoted != e.quoted {
            return Err(error("complete flag helper changed"));
        }
    }
    spans.insert("flag_primitive".into(), part_span(sources, PARSER, part)?);
    Ok((
        kind.ok_or_else(|| error("missing flag type"))?,
        value.ok_or_else(|| error("missing flag value"))?,
    ))
}

/// Prove actual functions before serializing the indirect path. The preserved
/// isolated constructor descriptor intentionally has no full-module select/type captures.
pub(super) fn verify_helper(
    lua: &Lua,
    function: &Function,
    id: ParserCallbackId,
    constructor: &Function,
    graph: &Graph<'_>,
    spans: &BTreeMap<String, ItemSourceSpan>,
) -> Result<()> {
    let constructor_id = graph
        .seen_callbacks
        .get(&(constructor.to_pointer() as usize))
        .ok_or_else(|| error("original flag constructor absent from graph"))?;
    let descriptor = graph
        .callbacks
        .get(
            id.0.checked_sub(1)
                .ok_or_else(|| error("invalid flag ID"))? as usize,
        )
        .ok_or_else(|| error("missing flag descriptor"))?;
    let constructor_descriptor = &graph.callbacks[constructor_id.0 as usize - 1];
    let actual: Function = lua.globals().get::<Table>("modLib")?.raw_get("createMod")?;
    let (name, captured): (Option<String>, Value) =
        graph.get_upvalue.call((function.clone(), 1))?;
    let (extra, _): (Option<String>, Value) = graph.get_upvalue.call((function.clone(), 2))?;
    let (constructor_capture, _): (Option<String>, Value) =
        graph.get_upvalue.call((constructor.clone(), 1))?;
    let Value::Function(captured) = captured else {
        return Err(error("flag mod capture is not a function"));
    };
    if graph.seen_callbacks.get(&(function.to_pointer() as usize)) != Some(&id)
        || actual.to_pointer() != constructor.to_pointer()
        || captured.to_pointer() != constructor.to_pointer()
        || name.as_deref() != Some("mod")
        || extra.is_some()
        || constructor_capture.is_some()
        || function
            .environment()
            .is_none_or(|e| e.to_pointer() != lua.globals().to_pointer())
        || constructor
            .environment()
            .is_none_or(|e| e.to_pointer() != lua.globals().to_pointer())
        || descriptor.kind
            != (ParserCallbackKind::Lua {
                source: spans["flag_primitive"].clone(),
            })
        || descriptor.upvalues
            != [ParserUpvalue {
                name: "mod".into(),
                value: ParserValue::Callback(*constructor_id),
            }]
        || constructor_descriptor.kind
            != (ParserCallbackKind::Lua {
                source: spans["create_mod"].clone(),
            })
        || !constructor_descriptor.upvalues.is_empty()
    {
        return Err(error(
            "flag does not capture the authenticated original constructor",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sources() -> BTreeMap<String, String> {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../vendor/path-of-building-poe2");
        [PARSER, TOOLS]
            .into_iter()
            .map(|p| {
                (
                    p.into(),
                    std::fs::read_to_string(root.join(p))
                        .unwrap()
                        .replace("\r\n", "\n"),
                )
            })
            .collect()
    }
    #[test]
    fn complete_flag_source_exports_literal_prefix_and_exact_span() {
        let original = sources();
        let mut spans = BTreeMap::new();
        assert_eq!(
            source_policy(&Lua::new(), &original, &mut spans).unwrap(),
            ("FLAG".into(), true)
        );
        assert_eq!(
            spans["flag_primitive"],
            line_span(&original, PARSER, 2177, 2179).unwrap()
        );
        for (kind, value) in [("", false), ("CustomType", true), ("a\0é", false)] {
            let mut changed = original.clone();
            // Use Lua's decimal escape for NUL; source literal decoding stays original.
            let quoted = format!("{kind:?}").replace("\\0", "\\000");
            changed.insert(
                PARSER.into(),
                original[PARSER].replacen(
                    "mod(name, \"FLAG\", true, ...)",
                    &format!("mod(name, {quoted}, {value}, ...)"),
                    1,
                ),
            );
            assert_eq!(
                source_policy(&Lua::new(), &changed, &mut BTreeMap::new()).unwrap(),
                (kind.into(), value)
            );
        }
    }
    #[test]
    fn flag_source_rejects_changed_body_literal_types_expansion_and_scope() {
        let original = sources();
        for (from, to) in [
            (
                "mod(name, \"FLAG\", true, ...)",
                "mod(name, \"FLAG\", true)",
            ),
            (
                "mod(name, \"FLAG\", true, ...)",
                "mod(name, \"FLAG\", true, (...))",
            ),
            (
                "mod(name, \"FLAG\", true, ...)",
                "mod(name, kind, true, ...)",
            ),
            ("mod(name, \"FLAG\", true, ...)", "mod(name, 1, true, ...)"),
            (
                "mod(name, \"FLAG\", true, ...)",
                "mod(name, \"FLAG\", 1, ...)",
            ),
            (
                "mod(name, \"FLAG\", true, ...)",
                "mod(name, \"FLAG\", \"true\", ...)",
            ),
            ("return mod(name,", "return other(name,"),
            ("flag(name, ...)", "flag(name, extra, ...)"),
            ("local function flag(", "function flag("),
        ] {
            let mut changed = original.clone();
            changed.insert(PARSER.into(), original[PARSER].replacen(from, to, 1));
            assert!(
                source_policy(&Lua::new(), &changed, &mut BTreeMap::new()).is_err(),
                "{to}"
            );
        }
    }
    #[test]
    fn actual_flag_constructor_capture_identity_environment_and_descriptor_are_proved() {
        let sources = sources();
        for case in 0..10 {
            // SAFETY: test-owned VM only; debug remains in this bounded test for negative upvalue controls.
            let lua = unsafe { Lua::unsafe_new() };
            let mut spans = BTreeMap::new();
            source_policy(&lua, &sources, &mut spans).unwrap();
            let constructor_part = top_function(&sources, TOOLS, "modLib.createMod").unwrap();
            let constructor_span = part_span(&sources, TOOLS, constructor_part).unwrap();
            lua.globals()
                .set("modLib", lua.create_table().unwrap())
                .unwrap();
            lua.load(format!(
                "{}{constructor_part}",
                "\n".repeat(constructor_span.line as usize - 1)
            ))
            .set_name(format!("@{TOOLS}"))
            .exec()
            .unwrap();
            spans.insert("create_mod".into(), constructor_span);
            let constructor: Function = lua
                .globals()
                .get::<Table>("modLib")
                .unwrap()
                .get("createMod")
                .unwrap();
            let helper_span = &spans["flag_primitive"];
            let helper_part = sources[PARSER]
                .split_inclusive('\n')
                .skip(helper_span.line as usize - 1)
                .take((helper_span.end_line - helper_span.line + 1) as usize)
                .collect::<String>();
            let helper: Function = lua
                .load(format!(
                    "{}local mod = modLib.createMod\n{helper_part}\nreturn flag",
                    "\n".repeat(helper_span.line as usize - 2)
                ))
                .set_name(format!("@{PARSER}"))
                .eval()
                .unwrap();
            let get_upvalue: Function = lua
                .globals()
                .get::<Table>("debug")
                .unwrap()
                .get("getupvalue")
                .unwrap();
            let mut graph = Graph {
                global_environment: lua.globals().to_pointer() as usize,
                sources: &sources,
                get_upvalue,
                builtins: BTreeMap::new(),
                seen_tables: BTreeMap::new(),
                seen_callbacks: BTreeMap::new(),
                tables: vec![],
                callbacks: vec![],
                values: 0,
            };
            let ParserValue::Callback(id) =
                graph.value(Value::Function(helper.clone()), 0).unwrap()
            else {
                panic!()
            };
            verify_helper(&lua, &helper, id, &constructor, &graph, &spans).unwrap();
            match case {
                0 => {
                    graph.callbacks[id.0 as usize - 1].upvalues[0].name = "other".into();
                }
                1 => {
                    graph.callbacks[id.0 as usize - 1]
                        .upvalues
                        .push(ParserUpvalue {
                            name: "extra".into(),
                            value: ParserValue::Nil,
                        });
                }
                2 => {
                    graph.callbacks[id.0 as usize - 1].upvalues[0].value =
                        ParserValue::Callback(id);
                }
                3 => {
                    graph
                        .seen_callbacks
                        .insert(helper.to_pointer() as usize, ParserCallbackId(99));
                }
                4 => {
                    helper.set_environment(lua.create_table().unwrap()).unwrap();
                }
                5 => {
                    constructor
                        .set_environment(lua.create_table().unwrap())
                        .unwrap();
                }
                6 => {
                    lua.globals()
                        .get::<Table>("modLib")
                        .unwrap()
                        .set(
                            "createMod",
                            lua.load("return function() end")
                                .eval::<Function>()
                                .unwrap(),
                        )
                        .unwrap();
                }
                7 => {
                    let setup: Function = lua
                        .globals()
                        .get::<Table>("debug")
                        .unwrap()
                        .get("setupvalue")
                        .unwrap();
                    setup
                        .call::<Value>((
                            helper.clone(),
                            1,
                            lua.load("return function() end")
                                .eval::<Function>()
                                .unwrap(),
                        ))
                        .unwrap();
                }
                8 => {
                    spans.get_mut("create_mod").unwrap().line += 1;
                }
                9 => {
                    spans.get_mut("flag_primitive").unwrap().end_line += 1;
                }
                _ => unreachable!(),
            }
            assert!(
                verify_helper(&lua, &helper, id, &constructor, &graph, &spans).is_err(),
                "case {case}"
            );
        }
    }
}
