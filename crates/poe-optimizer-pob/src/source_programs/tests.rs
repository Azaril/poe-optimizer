//! Lowering structure and binding tests; unchanged full-runtime parity is separate.
use super::*;
use poe_optimizer_data::item_loading::{ItemLoadingSource, ItemSourceSpan};
use poe_optimizer_data::source_program::{
    SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION, SourceProgramDefinitions, SourceProgramRoot,
};

const PATH: &str = "src/fixture.lua";
fn definitions(text: &str, callbacks: Vec<ParserCallback>) -> SourceProgramDefinitions {
    SourceProgramDefinitions {
        schema_version: SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION,
        source: ItemLoadingSource {
            upstream_revision: "a".repeat(40),
            files: [(PATH.into(), hash(text.as_bytes()))].into(),
            construction_spans: [("fixture".into(), span(text, 1, text.lines().count() as u32))]
                .into(),
            module_order: vec![PATH.into()],
        },
        tables: vec![ParserTable {
            fields: [("factor".into(), ParserValue::Number(3.0))].into(),
            indexed: BTreeMap::new(),
        }],
        callbacks,
        roots: vec![SourceProgramRoot {
            name: "Definitions".into(),
            table: ParserTableId(1),
        }],
        intrinsics: BTreeMap::new(),
    }
}
fn span(text: &str, line: u32, end_line: u32) -> ItemSourceSpan {
    ItemSourceSpan {
        path: PATH.into(),
        line,
        end_line,
        sha256: hash(
            text.split_inclusive('\n')
                .skip(line as usize - 1)
                .take((end_line - line + 1) as usize)
                .collect::<String>()
                .as_bytes(),
        ),
    }
}
fn callback(text: &str, line: u32, end_line: u32, upvalues: Vec<ParserUpvalue>) -> ParserCallback {
    ParserCallback {
        kind: ParserCallbackKind::Lua {
            source: span(text, line, end_line),
        },
        upvalues,
        environment: ParserEnvironment::OriginalGlobals,
    }
}
fn sources(text: &str) -> BTreeMap<String, String> {
    [(PATH.into(), text.into())].into()
}

#[test]
fn named_definition_roots_are_explicit_and_exact_owner_bound() {
    let text = "function(value) return value * Definitions.factor end\n";
    let owner =
        SourceProgramOwner::new(definitions(text, vec![callback(text, 1, 1, vec![])])).unwrap();
    let lowered = lower_from_sources(&sources(text), &owner).unwrap();
    assert!(lowered.unsupported().is_empty());
    assert!(lowered.catalog().is_bound_to(&owner));
    let program = &lowered.catalog().data().programs[0];
    assert_eq!(program.parameter_count, 1);
    assert_eq!(program.provenance.source, span(text, 1, 1));
    let ParserProgramStatementKind::Return { values } = &program.body[0].operation else {
        panic!("return")
    };
    let ParserProgramExprKind::Binary { right, .. } = &values.values[0].operation else {
        panic!("multiply")
    };
    let ParserProgramExprKind::Get { table, .. } = &right.operation else {
        panic!("lookup")
    };
    assert!(matches!(
        table.operation,
        ParserProgramExprKind::NamedDefinition { .. }
    ));
    let mut stale = definitions(text, vec![callback(text, 1, 1, vec![])]);
    stale.roots[0].name = "OtherDefinitions".into();
    let rejected =
        lower_from_sources(&sources(text), &SourceProgramOwner::new(stale).unwrap()).unwrap();
    assert!(rejected.catalog().data().programs.is_empty());
    assert!(rejected.unsupported()[&ParserCallbackId(1)].contains("Definitions"));
}

#[test]
fn captured_call_closure_is_complete_and_unsupported_dependencies_propagate() {
    let text = "local function helper(value) return value + 2 end\nfunction(value) return helper(value) end\n";
    let owner = SourceProgramOwner::new(definitions(
        text,
        vec![
            callback(text, 1, 1, vec![]),
            callback(
                text,
                2,
                2,
                vec![ParserUpvalue {
                    name: "helper".into(),
                    value: ParserValue::Callback(ParserCallbackId(1)),
                }],
            ),
        ],
    ))
    .unwrap();
    let lowered = lower_from_sources(&sources(text), &owner).unwrap();
    assert!(lowered.unsupported().is_empty());
    assert_eq!(lowered.catalog().data().programs.len(), 2);
    assert!(matches!(
        lowered.catalog().data().programs[1].bindings[0],
        ParserProgramBinding::CapturedCallback {
            callback: ParserCallbackId(1),
            upvalue: 0
        }
    ));
    let text = "local function helper(value) if false then while true do end end return value end\nfunction(value) return helper(value) end\n";
    let owner = SourceProgramOwner::new(definitions(
        text,
        vec![
            callback(text, 1, 1, vec![]),
            callback(
                text,
                2,
                2,
                vec![ParserUpvalue {
                    name: "helper".into(),
                    value: ParserValue::Callback(ParserCallbackId(1)),
                }],
            ),
        ],
    ))
    .unwrap();
    let rejected = lower_from_sources(&sources(text), &owner).unwrap();
    assert!(rejected.catalog().data().programs.is_empty());
    assert_eq!(rejected.unsupported().len(), 2);
    assert!(rejected.unsupported()[&ParserCallbackId(2)].contains("no complete lowered program"));
}

#[test]
fn implicit_self_and_captured_intrinsic_use_declared_owner_bindings() {
    let text = "function List:AddMod(mod) t_insert(self, mod) end\n";
    let mut data = definitions(
        text,
        vec![
            callback(
                text,
                1,
                1,
                vec![ParserUpvalue {
                    name: "t_insert".into(),
                    value: ParserValue::Callback(ParserCallbackId(2)),
                }],
            ),
            ParserCallback {
                kind: ParserCallbackKind::Builtin {
                    symbol: "table.insert".into(),
                },
                upvalues: vec![],
                environment: ParserEnvironment::OriginalGlobals,
            },
        ],
    );
    let unbound = lower_from_sources(
        &sources(text),
        &SourceProgramOwner::new(data.clone()).unwrap(),
    )
    .unwrap();
    assert!(unbound.catalog().data().programs.is_empty());
    data.intrinsics
        .insert(ParserCallbackId(2), ParserProgramIntrinsic::TableInsert);
    let owner = SourceProgramOwner::new(data).unwrap();
    let lowered = lower_from_sources(&sources(text), &owner).unwrap();
    assert!(lowered.unsupported().is_empty());
    let program = &lowered.catalog().data().programs[0];
    assert_eq!(program.parameter_count, 2);
    assert_eq!(program.local_count, 2);
    assert!(matches!(
        program.bindings[0],
        ParserProgramBinding::Intrinsic {
            operation: ParserProgramIntrinsic::TableInsert,
            source: ParserProgramIntrinsicSource::Captured {
                callback: ParserCallbackId(2),
                upvalue: 0
            }
        }
    ));
    // No static dispatch shortcut is substituted for an actual receiver lookup.
    let method = "function List:NewMod(mod) self:AddMod(mod) end\n";
    let rejected = lower_from_sources(
        &sources(method),
        &SourceProgramOwner::new(definitions(method, vec![callback(method, 1, 1, vec![])]))
            .unwrap(),
    )
    .unwrap();
    assert!(rejected.catalog().data().programs.is_empty());
    assert!(rejected.unsupported()[&ParserCallbackId(1)].contains("unsupported method AddMod"));
}

#[test]
fn exact_source_inventory_and_spans_are_checked_before_lowering() {
    let text = "function(value) return tonumber(value) end\n";
    let data = definitions(text, vec![callback(text, 1, 1, vec![])]);
    let owner = SourceProgramOwner::new(data.clone()).unwrap();
    assert!(lower_from_sources(&BTreeMap::new(), &owner).is_err());
    assert!(lower_from_sources(&sources(&text.replace("tonumber", "tostring")), &owner).is_err());
    let mut extra = sources(text);
    extra.insert("unexpected.lua".into(), "".into());
    assert!(lower_from_sources(&extra, &owner).is_err());
    let mut data = data;
    let ParserCallbackKind::Lua { source } = &mut data.callbacks[0].kind else {
        panic!("lua")
    };
    source.sha256 = "0".repeat(64);
    assert!(lower_from_sources(&sources(text), &SourceProgramOwner::new(data).unwrap()).is_err());
}

#[test]
fn shadowed_globals_and_undeclared_game_roots_do_not_gain_authority() {
    for (text, upvalues) in [
        (
            "function(value) return tonumber(value) end\n",
            vec![ParserUpvalue {
                name: "tonumber".into(),
                value: ParserValue::Number(7.0),
            }],
        ),
        ("function() return ModFlag.Attack end\n", vec![]),
        ("function(value) return os.execute(value) end\n", vec![]),
    ] {
        let owner =
            SourceProgramOwner::new(definitions(text, vec![callback(text, 1, 1, upvalues)]))
                .unwrap();
        let rejected = lower_from_sources(&sources(text), &owner).unwrap();
        assert!(rejected.catalog().data().programs.is_empty(), "{text}");
        assert_eq!(rejected.unsupported().len(), 1);
    }
}
