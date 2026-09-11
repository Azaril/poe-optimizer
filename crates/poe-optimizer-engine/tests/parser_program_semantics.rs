//! Independent interpreted LuaJIT controls and raw native program parity.
//! Explicitly authored IR is compared with actual Lua constructs and the original
//! modifier constructor. JIT is disabled; no warmed traces, automatic source
//! lowering, callback admission or whole-build parity is claimed.
#![cfg(not(target_arch = "wasm32"))]
use mlua::{Function, Lua, MultiValue, Table, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Boolean(bool),
    Number(u64),
    Bytes(Vec<u8>),
}
#[derive(Debug, Clone, PartialEq, Eq)]
enum Atom {
    Nil,
    Boolean(bool),
    Number(u64),
    Bytes(Vec<u8>),
    Table(usize),
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct Graph {
    results: Vec<Atom>,
    tables: Vec<Vec<(Key, Atom)>>,
}
fn number(v: f64) -> Atom {
    Atom::Number(v.to_bits())
}
fn bytes(v: &[u8]) -> Atom {
    Atom::Bytes(v.to_vec())
}
fn source() -> Lua {
    let lua = Lua::new();
    lua.load("jit.off(); jit.flush(); assert(not jit.status())")
        .exec()
        .unwrap();
    lua
}
fn key(value: Value) -> Key {
    match value {
        Value::Boolean(v) => Key::Boolean(v),
        Value::Integer(v) => Key::Number((v as f64).to_bits()),
        Value::Number(v) => Key::Number(if v == 0.0 { 0 } else { v.to_bits() }),
        Value::String(v) => Key::Bytes(v.as_bytes().to_vec()),
        v => panic!("unrepresented source key {}", v.type_name()),
    }
}
fn intern(value: Value, ids: &mut BTreeMap<usize, usize>, tables: &mut Vec<Table>) -> Atom {
    match value {
        Value::Nil => Atom::Nil,
        Value::Boolean(v) => Atom::Boolean(v),
        Value::Integer(v) => number(v as f64),
        Value::Number(v) => number(v),
        Value::String(v) => bytes(v.as_bytes().as_ref()),
        Value::Table(t) => {
            let pointer = t.to_pointer() as usize;
            let index = if let Some(id) = ids.get(&pointer) {
                *id
            } else {
                assert!(tables.len() < 256, "source graph observation bound");
                let id = tables.len();
                ids.insert(pointer, id);
                tables.push(t);
                id
            };
            Atom::Table(index)
        }
        v => panic!("unrepresented source value {}", v.type_name()),
    }
}
fn capture(results: MultiValue) -> Graph {
    let mut ids = BTreeMap::new();
    let mut pending = Vec::new();
    let results = results
        .into_iter()
        .map(|v| intern(v, &mut ids, &mut pending))
        .collect();
    let mut tables = Vec::new();
    while tables.len() < pending.len() {
        let t = pending[tables.len()].clone();
        let mut fields = t
            .pairs::<Value, Value>()
            .map(|p| {
                let (k, v) = p.unwrap();
                (key(k), v)
            })
            .collect::<Vec<_>>();
        fields.sort_by(|a, b| a.0.cmp(&b.0));
        tables.push(
            fields
                .into_iter()
                .map(|(k, v)| (k, intern(v, &mut ids, &mut pending)))
                .collect(),
        );
    }
    Graph { results, tables }
}
fn sequence(t: Table) -> Vec<Atom> {
    t.sequence_values::<Value>()
        .map(|v| {
            let mut pending = Vec::new();
            let a = intern(v.unwrap(), &mut BTreeMap::new(), &mut pending);
            assert!(pending.is_empty());
            a
        })
        .collect()
}
fn numeric_for(lua: &Lua) -> Function {
    lua.load(
        r#"
      return function(a,b,c,mutate)
        local seen,events={},{}
        local function operand(label,value) events[#events+1]=label;return value end
        local ok,err=pcall(function()
          for i=operand('start',a),operand('limit',b),operand('step',c) do
            seen[#seen+1]=i
            if mutate then i=999 end
            if #seen==4 then break end
          end
        end)
        return ok,seen,events,err
      end
    "#,
    )
    .set_name("@parser-program-numeric-for-source")
    .eval()
    .unwrap()
}

#[test]
fn interpreted_numeric_for_uses_hidden_control_and_signed_step_direction() {
    let lua = source();
    let f = numeric_for(&lua);
    let cases: &[(&str, f64, f64, f64, &[f64])] = &[
        ("positive", 1., 3., 1., &[1., 2., 3.]),
        ("negative", 3., 1., -1., &[3., 2., 1.]),
        ("zero_up", 1., 2., 0., &[1., 1., 1., 1.]),
        ("zero_equal", 1., 1., 0., &[1., 1., 1., 1.]),
        ("zero_down", 2., 1., 0., &[]),
        ("negative_zero_up", 1., 2., -0., &[]),
        ("negative_zero_down", 2., 1., -0., &[2., 2., 2., 2.]),
        ("negative_initial_zero", -0., 0., 1., &[-0.]),
        ("nan_start", f64::NAN, 2., 1., &[]),
        ("nan_limit", 1., f64::NAN, 1., &[]),
        ("nan_step_ascending", 1., 2., f64::NAN, &[]),
        ("nan_step_descending", 2., 1., f64::NAN, &[2.]),
        ("infinite_step", 1., 2., f64::INFINITY, &[1.]),
        (
            "infinite_equal",
            f64::INFINITY,
            f64::INFINITY,
            1.,
            &[f64::INFINITY; 4],
        ),
        ("negative_infinite_step", 2., 1., f64::NEG_INFINITY, &[2.]),
    ];
    for (label, a, b, c, expected) in cases {
        let (ok, seen, events, err): (bool, Table, Table, Value) =
            f.call((*a, *b, *c, true)).unwrap();
        assert!(ok, "{label}: {err:?}");
        assert_eq!(
            sequence(seen),
            expected.iter().copied().map(number).collect::<Vec<_>>(),
            "{label}"
        );
        assert_eq!(
            sequence(events),
            vec![bytes(b"start"), bytes(b"limit"), bytes(b"step")]
        );
        assert!(matches!(err, Value::Nil));
    }
}

#[test]
fn interpreted_numeric_for_evaluates_all_operands_before_ordered_coercion() {
    let lua = source();
    let f = numeric_for(&lua);
    for (a, b, c, message) in [
        (
            Value::Boolean(false),
            Value::Boolean(false),
            Value::Boolean(false),
            "'for' initial value must be a number",
        ),
        (
            Value::Number(1.),
            Value::Boolean(false),
            Value::Boolean(false),
            "'for' limit must be a number",
        ),
        (
            Value::Number(1.),
            Value::Number(2.),
            Value::Boolean(false),
            "'for' step must be a number",
        ),
    ] {
        let (ok, seen, events, err): (bool, Table, Table, String) =
            f.call((a, b, c, false)).unwrap();
        assert!(!ok);
        assert!(sequence(seen).is_empty());
        assert_eq!(
            sequence(events),
            vec![bytes(b"start"), bytes(b"limit"), bytes(b"step")]
        );
        assert!(err.ends_with(message), "{err}");
    }
    let (ok, seen, _, _): (bool, Table, Table, Value) = f.call(("1", "3", "1", false)).unwrap();
    assert!(ok);
    assert_eq!(sequence(seen), vec![number(1.), number(2.), number(3.)]);
    let (ok, events, err): (bool, Table, String) = lua
        .load(
            r#"
      local events={}
      local function operand(label,value) events[#events+1]=label;return value end
      local ok,err=pcall(function()
        for i=operand('start',false),error('limit expression'),operand('step',false) do end
      end)
      return ok,events,err
    "#,
        )
        .eval()
        .unwrap();
    assert!(!ok);
    assert_eq!(sequence(events), vec![bytes(b"start")]);
    assert!(err.ends_with("limit expression"));
}

#[test]
fn interpreted_gmatch_preserves_capture_packs_empty_progress_and_literal_caret() {
    let lua = source();
    let make: Function = lua.load("return string.gmatch").eval().unwrap();
    type MatchCase<'a> = (&'a [u8], &'a [u8], Vec<Vec<Atom>>);
    let cases: Vec<MatchCase<'_>> = vec![
        (b"ab", b"", vec![vec![bytes(b"")]; 3]),
        (b"a", b"a*", vec![vec![bytes(b"a")], vec![bytes(b"")]]),
        (
            b"ab",
            b"()",
            vec![vec![number(1.)], vec![number(2.)], vec![number(3.)]],
        ),
        (
            b"ab",
            b"()(.)",
            vec![vec![number(1.), bytes(b"a")], vec![number(2.), bytes(b"b")]],
        ),
        (b"^a", b"^", vec![vec![bytes(b"^")]]),
        (b"ab", b"$", vec![vec![bytes(b"")]]),
        (b"ab", b"(.-)", vec![vec![bytes(b"")]; 3]),
        (b"", b"a*", vec![vec![bytes(b"")]]),
        (
            b"a\0\xff",
            b"(.)",
            vec![vec![bytes(b"a")], vec![bytes(b"\0")], vec![bytes(b"\xff")]],
        ),
    ];
    for (text, pattern, expected) in cases {
        let iter: Function = make
            .call((
                lua.create_string(text).unwrap(),
                lua.create_string(pattern).unwrap(),
            ))
            .unwrap();
        for row in expected {
            assert_eq!(
                capture(iter.call(()).unwrap()),
                Graph {
                    results: row,
                    tables: vec![]
                },
                "{text:?}/{pattern:?}"
            );
        }
        for _ in 0..2 {
            let end: MultiValue = iter.call(()).unwrap();
            assert!(end.is_empty());
        }
    }
}

#[test]
fn interpreted_gmatch_delays_pattern_errors_but_checks_receiver_before_arguments() {
    let lua = source();
    let make: Function = lua.load("return string.gmatch").eval().unwrap();
    let iter: Function = make.call(("ab", "[")).unwrap();
    assert!(
        iter.call::<MultiValue>(())
            .unwrap_err()
            .to_string()
            .contains("malformed pattern")
    );
    let (ok, events, err): (bool, Table, String) = lua
        .load(
            r#"
      local events={}
      local function arg() events[#events+1]='argument';return '[' end
      local receiver=12
      local ok,err=pcall(function() return receiver:gmatch(arg()) end)
      return ok,events,err
    "#,
        )
        .eval()
        .unwrap();
    assert!(!ok);
    assert!(sequence(events).is_empty());
    assert!(err.contains("attempt to index"));
    let iter: Function = make.call((123, 2)).unwrap();
    assert_eq!(capture(iter.call(()).unwrap()).results, vec![bytes(b"2")]);
}

#[test]
fn interpreted_ipairs_stops_at_first_nil_and_uses_hidden_index_with_live_table_reads() {
    let lua = source();
    let (rows,index_calls):(Table,i64)=lua.load(r#"
      local calls=0
      local t=setmetatable({'a',false,[4]='ignored'}, {__index=function() calls=calls+1;return 'not read' end})
      local rows={}
      for i,v in ipairs(t) do rows[#rows+1]={i,v};i=100;v='changed' end
      return rows,calls
    "#).eval().unwrap();
    assert_eq!(index_calls, 0);
    let rows = rows
        .sequence_values::<Table>()
        .map(|t| sequence(t.unwrap()))
        .collect::<Vec<_>>();
    assert_eq!(
        rows,
        vec![
            vec![number(1.), bytes(b"a")],
            vec![number(2.), Atom::Boolean(false)]
        ]
    );
    let rows: Table = lua
        .load(
            r#"
      local t={'a'}; local rows={}
      for i,v in ipairs(t) do
        rows[#rows+1]={i,v}
        if i<3 then t[i+1]='added' end
      end
      return rows
    "#,
        )
        .eval()
        .unwrap();
    let rows = rows
        .sequence_values::<Table>()
        .map(|t| sequence(t.unwrap()))
        .collect::<Vec<_>>();
    assert_eq!(
        rows,
        vec![
            vec![number(1.), bytes(b"a")],
            vec![number(2.), bytes(b"added")],
            vec![number(3.), bytes(b"added")]
        ]
    );
    let make: Function = lua.load("return ipairs").eval().unwrap();
    assert!(make.call::<MultiValue>(false).is_err());
    assert!(make.call::<MultiValue>("ab").is_err());
}

#[test]
fn interpreted_constructor_constant_templates_and_dynamic_writes_are_distinct() {
    let lua = source();
    let cases: &[(&str, &[(u32, &str)])] = &[
        ("{[1]='key','list'}", &[(1, "list")]),
        ("{'list',[1]='key'}", &[(1, "key")]),
        ("{[1]='a','b',[1]='c','d'}", &[(1, "c"), (2, "d")]),
        ("{'a',nil,'c',[2]='key'}", &[(1, "a"), (2, "key"), (3, "c")]),
        (
            "{[1]='key',(function() return nil,'tail' end)()}",
            &[(2, "tail")],
        ),
        (
            "{'first',[1]='key',unpack({'tail','last'})}",
            &[(1, "key"), (2, "tail"), (3, "last")],
        ),
    ];
    for (expression, expected) in cases {
        let result: MultiValue = lua.load(format!("return {expression}")).eval().unwrap();
        let table = expected
            .iter()
            .map(|(k, v)| (Key::Number((*k as f64).to_bits()), bytes(v.as_bytes())))
            .collect();
        assert_eq!(
            capture(result),
            Graph {
                results: vec![Atom::Table(0)],
                tables: vec![table]
            },
            "{expression}"
        );
    }
    // Original LuaJIT lj_parse.c expr_table hoists constant key/value stores into
    // a TDUP template. A subsequent dynamic store can overwrite a later literal.
    // A source lowerer must model that initialization or defer these collisions.
    for expression in [
        "{[1]=tonumber('2'),[1]=3}",
        "{tonumber('2'),[1]=3}",
        "{[1]=tonumber('2'),[1]=nil}",
    ] {
        let result: MultiValue = lua.load(format!("return {expression}")).eval().unwrap();
        assert_eq!(
            capture(result),
            Graph {
                results: vec![Atom::Table(0)],
                tables: vec![vec![(Key::Number(1f64.to_bits()), number(2.))]]
            },
            "{expression}"
        );
    }
    let result: MultiValue = lua
        .load("return {name=tonumber('2'),name=3}")
        .eval()
        .unwrap();
    assert_eq!(
        capture(result),
        Graph {
            results: vec![Atom::Table(0)],
            tables: vec![vec![(Key::Bytes(b"name".to_vec()), number(2.))]]
        }
    );
}

#[test]
fn interpreted_constructor_errors_follow_key_and_value_evaluation_order() {
    let lua = source();
    let (ok, events, err): (bool, Table, String) = lua
        .load(
            r#"
      local events={}
      local function key() events[#events+1]='key';return nil end
      local function value() events[#events+1]='value';return 3 end
      local ok,err=pcall(function() return {[key()]=value()} end)
      return ok,events,err
    "#,
        )
        .eval()
        .unwrap();
    assert!(!ok);
    assert_eq!(sequence(events), vec![bytes(b"key"), bytes(b"value")]);
    assert!(err.contains("table index is nil"));
    let (t, events): (Table, Table) = lua
        .load(
            r#"
      local events={}
      local function value(name) events[#events+1]=name;return name end
      return {[1]=value('first'),value('second'),[1]=value('third')},events
    "#,
        )
        .eval()
        .unwrap();
    assert_eq!(
        sequence(events),
        vec![bytes(b"first"), bytes(b"second"), bytes(b"third")]
    );
    assert_eq!(sequence(t), vec![bytes(b"third")]);
}

#[test]
fn interpreted_raw_results_preserve_zero_nil_trailing_nil_signed_zero_and_alias_graph() {
    let lua = source();
    for (body, expected) in [
        ("return", vec![]),
        ("return nil", vec![Atom::Nil]),
        ("return 1,nil", vec![number(1.), Atom::Nil]),
        ("return nil,nil,nil", vec![Atom::Nil; 3]),
        ("return -0,0", vec![number(-0.), number(0.)]),
    ] {
        let f: Function = lua
            .load(format!("return function() {body} end"))
            .eval()
            .unwrap();
        assert_eq!(
            capture(f.call(()).unwrap()),
            Graph {
                results: expected,
                tables: vec![]
            },
            "{body}"
        );
    }
    let f: Function = lua
        .load("return function() local t={};t.self=t;return t,t,{ref=t} end")
        .eval()
        .unwrap();
    assert_eq!(
        capture(f.call(()).unwrap()),
        Graph {
            results: vec![Atom::Table(0), Atom::Table(0), Atom::Table(1)],
            tables: vec![
                vec![(Key::Bytes(b"self".to_vec()), Atom::Table(0))],
                vec![(Key::Bytes(b"ref".to_vec()), Atom::Table(0))]
            ],
        }
    );
}

#[path = "support/parser_program_native.rs"]
mod native;

#[test]
fn raw_numeric_for_ir_matches_source_hidden_control_and_ieee_boundaries() {
    use native::*;
    use poe_optimizer_engine::parser_program::{ProgramRuntimeErrorKind, ProgramValue as V};
    let plan = compile(
        3,
        false,
        vec![],
        vec![
            declare(3, table(vec![])),
            s(S::ForNumeric {
                local: 4,
                start: l(0),
                limit: l(1),
                step: l(2),
                body: vec![
                    append(l(3), l(4)),
                    s(S::Assign {
                        locals: vec![4],
                        values: values(vec![n(999.)]),
                    }),
                    branch(
                        binary(B::Equal, unary(U::Length, l(3)), n(4.)),
                        vec![s(S::Break)],
                    ),
                ],
            }),
            ret(vec![l(3)]),
        ],
    );
    let lua = source();
    let f:Function=lua.load("return function(a,b,c) local t={};for i=a,b,c do t[#t+1]=i;i=999;if #t==4 then break end end;return t end").eval().unwrap();
    let nan = f64::from_bits(0xfff8000000000000);
    for (a, b, c) in [
        (1., 3., 1.),
        (3., 1., -1.),
        (1., 2., 0.),
        (2., 1., 0.),
        (1., 2., -0.),
        (2., 1., -0.),
        (-0., 0., 1.),
        (nan, 2., 1.),
        (1., nan, 1.),
        (1., 2., nan),
        (2., 1., nan),
        (1., 2., f64::INFINITY),
        (f64::INFINITY, f64::INFINITY, 1.),
    ] {
        let original = capture(f.call((a, b, c)).unwrap());
        let output = execute(
            &plan,
            &input(vec![V::Number(a), V::Number(b), V::Number(c)]),
        )
        .unwrap();
        assert_eq!(graph(output.graph()), original, "{a:?}/{b:?}/{c:?}");
    }
    for (a, b, c, field) in [
        (
            Value::Boolean(false),
            Value::Number(2.),
            Value::Number(1.),
            "initial",
        ),
        (
            Value::Number(1.),
            Value::Boolean(false),
            Value::Number(1.),
            "limit",
        ),
        (
            Value::Number(1.),
            Value::Number(2.),
            Value::Boolean(false),
            "step",
        ),
    ] {
        let args = MultiValue::from_vec(vec![a, b, c]);
        let incoming = from_source(&capture(args.clone()));
        let original = f.call::<MultiValue>(args).unwrap_err().to_string();
        let native = execute(&plan, &incoming).unwrap_err();
        assert_eq!(native.kind, ProgramRuntimeErrorKind::Source);
        assert!(original.contains(field) && native.message.contains(field));
    }
}

#[test]
fn raw_nested_numeric_and_dense_loops_match_breaks_and_live_reads() {
    use native::*;
    use poe_optimizer_engine::parser_program::ProgramValueGraph;
    let lua = source();
    let plan = compile(
        0,
        false,
        vec![binding(I::Ipairs)],
        vec![
            declare(0, table(vec![list(text(b"first"))])),
            declare(1, table(vec![])),
            s(S::ForEach {
                locals: vec![2, 3],
                iterator: poe_optimizer_data::modifier_parser::ParserProgramIterator::Dense {
                    table: l(0),
                    binding: 0,
                },
                body: vec![
                    append(l(1), table(vec![list(l(2)), list(l(3))])),
                    branch(
                        binary(B::LessThan, l(2), n(3.)),
                        vec![set(l(0), binary(B::Add, l(2), n(1.)), text(b"added"))],
                    ),
                    s(S::Assign {
                        locals: vec![2],
                        values: values(vec![n(999.)]),
                    }),
                ],
            }),
            ret(vec![l(0), l(1)]),
        ],
    );
    let original:MultiValue=lua.load("local t={'first'};local r={};for i,v in ipairs(t) do r[#r+1]={i,v};if i<3 then t[i+1]='added' end;i=999 end;return t,r").eval().unwrap();
    assert_eq!(
        graph(
            execute(&plan, &ProgramValueGraph::default())
                .unwrap()
                .graph()
        ),
        capture(original)
    );
    let plan = compile(
        0,
        false,
        vec![],
        vec![
            declare(0, table(vec![])),
            s(S::ForNumeric {
                local: 1,
                start: n(1.),
                limit: n(3.),
                step: n(1.),
                body: vec![
                    s(S::ForNumeric {
                        local: 2,
                        start: n(1.),
                        limit: n(3.),
                        step: n(1.),
                        body: vec![
                            append(l(0), table(vec![list(l(1)), list(l(2))])),
                            s(S::Break),
                        ],
                    }),
                    branch(binary(B::Equal, l(1), n(2.)), vec![s(S::Break)]),
                ],
            }),
            ret(vec![l(0)]),
        ],
    );
    let original:MultiValue=lua.load("local r={};for i=1,3 do for j=1,3 do r[#r+1]={i,j};break end;if i==2 then break end end;return r").eval().unwrap();
    assert_eq!(
        graph(
            execute(&plan, &ProgramValueGraph::default())
                .unwrap()
                .graph()
        ),
        capture(original)
    );
}

#[test]
fn raw_pattern_loop_matches_source_capture_packs_empty_progress_and_errors() {
    use native::*;
    use poe_optimizer_data::modifier_parser::ParserProgramIterator;
    use poe_optimizer_engine::parser_program::{ProgramRuntimeErrorKind, ProgramValue as V};
    let plan = compile(
        2,
        false,
        vec![binding(I::StringGmatch)],
        vec![
            declare(2, table(vec![])),
            s(S::ForEach {
                locals: vec![3, 4],
                iterator: ParserProgramIterator::Pattern {
                    call: call(0, vec![l(0), l(1)]),
                },
                body: vec![append(l(2), table(vec![list(l(3)), list(l(4))]))],
            }),
            ret(vec![l(2)]),
        ],
    );
    let lua = source();
    let f:Function=lua.load("return function(s,p) local r={};for a,b in string.gmatch(s,p) do r[#r+1]={a,b} end;return r end").eval().unwrap();
    for (text, pattern) in [
        (b"ab".as_slice(), b"".as_slice()),
        (b"a", b"a*"),
        (b"ab", b"()"),
        (b"ab", b"()(.)"),
        (b"^a", b"^"),
        (b"ab", b"$"),
        (b"a\0\xff", b"(.)"),
    ] {
        let original = capture(
            f.call((
                lua.create_string(text).unwrap(),
                lua.create_string(pattern).unwrap(),
            ))
            .unwrap(),
        );
        let output = execute(
            &plan,
            &input(vec![V::Bytes(text.to_vec()), V::Bytes(pattern.to_vec())]),
        )
        .unwrap();
        assert_eq!(graph(output.graph()), original, "{text:?}/{pattern:?}");
    }
    let error = execute(
        &plan,
        &input(vec![V::Bytes(b"ab".to_vec()), V::Bytes(b"[".to_vec())]),
    )
    .unwrap_err();
    assert_eq!(error.kind, ProgramRuntimeErrorKind::Source);
    assert!(f.call::<MultiValue>(("ab", "[")).is_err());
}

#[test]
fn explicitly_template_lowered_ir_matches_original_constructor_collisions() {
    use native::*;
    use poe_optimizer_engine::parser_program::ProgramValue as V;
    let lua = source();
    for named in [false, true] {
        let key = if named { text(b"name") } else { n(1.) };
        let fields = if named {
            vec![F::Named {
                key: "name".into(),
                value: n(3.),
            }]
        } else {
            vec![F::Keyed {
                key: n(1.),
                value: n(3.),
            }]
        };
        let plan = compile(
            1,
            false,
            vec![binding(I::ToNumber)],
            vec![
                declare(1, table(fields)),
                set(
                    l(1),
                    key,
                    e(E::Call {
                        call: Box::new(call(0, vec![l(0)])),
                    }),
                ),
                ret(vec![l(1)]),
            ],
        );
        let f: Function = lua
            .load(if named {
                "return function(v) return {name=tonumber(v),name=3} end"
            } else {
                "return function(v) return {[1]=tonumber(v),[1]=3} end"
            })
            .eval()
            .unwrap();
        for value in ["2", "bad", "-0", "nan"] {
            let original = capture(f.call(value).unwrap());
            let output = execute(&plan, &input(vec![V::Bytes(value.as_bytes().to_vec())])).unwrap();
            assert_eq!(graph(output.graph()), original, "{named}/{value}");
        }
    }
}

#[test]
fn raw_result_packs_varargs_and_mutable_aliases_match_source() {
    use native::*;
    use poe_optimizer_data::modifier_parser::*;
    let lua = source();
    for (body, statements) in [
        ("return", vec![]),
        ("return nil", vec![ret(vec![nil()])]),
        ("return 1,nil", vec![ret(vec![n(1.), nil()])]),
        ("return -0,0", vec![ret(vec![n(-0.), n(0.)])]),
    ] {
        let plan = compile(0, false, vec![], statements);
        let original: MultiValue = lua.load(body).eval().unwrap();
        assert_eq!(
            graph(execute(&plan, &Default::default()).unwrap().graph()),
            capture(original)
        );
    }
    let plan = compile(
        1,
        true,
        vec![],
        vec![s(S::Return {
            values: ParserProgramValueList {
                values: vec![l(0)],
                tail: Some(Box::new(ParserProgramPack::Varargs)),
            },
        })],
    );
    let f: Function = lua
        .load("return function(a,...) return a,... end")
        .eval()
        .unwrap();
    for args in [
        MultiValue::new(),
        MultiValue::from_vec(vec![Value::Nil]),
        MultiValue::from_vec(vec![
            Value::Number(-0.),
            Value::Boolean(false),
            Value::Nil,
            Value::Nil,
        ]),
    ] {
        let incoming = from_source(&capture(args.clone()));
        assert_eq!(
            graph(execute(&plan, &incoming).unwrap().graph()),
            capture(f.call(args).unwrap())
        );
    }
    let plan = compile(
        0,
        false,
        vec![],
        vec![
            declare(0, table(vec![])),
            declare(1, l(0)),
            set(l(1), text(b"self"), l(0)),
            ret(vec![
                l(0),
                l(1),
                table(vec![F::Named {
                    key: "ref".into(),
                    value: l(0),
                }]),
            ]),
        ],
    );
    let original: MultiValue = lua
        .load("local t={};local alias=t;alias.self=t;return t,alias,{ref=t}")
        .eval()
        .unwrap();
    assert_eq!(
        graph(execute(&plan, &Default::default()).unwrap().graph()),
        capture(original)
    );
}

#[test]
fn raw_intrinsic_packs_match_tonumber_and_gsub_without_public_parser_adjustment() {
    use native::*;
    use poe_optimizer_data::modifier_parser::*;
    let lua = source();
    for (intrinsic, source_function, cases) in [
        (
            I::ToNumber,
            "return function(...) return tonumber(...) end",
            vec![
                vec![Value::Nil],
                vec![Value::Boolean(false)],
                vec![Value::String(lua.create_string("-0").unwrap())],
                vec![Value::String(lua.create_string("nan").unwrap())],
            ],
        ),
        (
            I::StringGsub,
            "return function(...) return string.gsub(...) end",
            vec![
                vec![
                    Value::String(lua.create_string("aba").unwrap()),
                    Value::String(lua.create_string("a").unwrap()),
                    Value::String(lua.create_string("x").unwrap()),
                ],
                vec![
                    Value::String(lua.create_string("a").unwrap()),
                    Value::String(lua.create_string("").unwrap()),
                    Value::String(lua.create_string("-").unwrap()),
                ],
            ],
        ),
    ] {
        let plan = compile(
            0,
            true,
            vec![binding(intrinsic)],
            vec![s(S::Return {
                values: ParserProgramValueList {
                    values: vec![],
                    tail: Some(Box::new(ParserProgramPack::Call {
                        call: ParserProgramCall {
                            binding: 0,
                            receiver: None,
                            arguments: ParserProgramValueList {
                                values: vec![],
                                tail: Some(Box::new(ParserProgramPack::Varargs)),
                            },
                        },
                    })),
                },
            })],
        );
        let f: Function = lua.load(source_function).eval().unwrap();
        for args in cases {
            let args = MultiValue::from_vec(args);
            let incoming = from_source(&capture(args.clone()));
            assert_eq!(
                graph(execute(&plan, &incoming).unwrap().graph()),
                capture(f.call(args).unwrap())
            );
        }
    }
}

fn original_create_mod(lua: &Lua) -> Function {
    use sha2::{Digest, Sha256};
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vendor/path-of-building-poe2/src/Modules/ModTools.lua"
    ))
    .unwrap()
    .replace("\r\n", "\n");
    assert_eq!(
        format!("{:x}", Sha256::digest(source.as_bytes())),
        "1ebd614ca55c052cb0be6dd0d16c8b9a944540a47ad50eca607c21d8913e1246"
    );
    // Retain unchanged original lexical primitive declarations and the complete
    // constructor body. No callback introspection/opaque closure parity claim.
    let prefix = source.lines().take(18).collect::<Vec<_>>().join("\n");
    let body = source
        .lines()
        .skip(56)
        .take(35)
        .collect::<Vec<_>>()
        .join("\n");
    lua.load(format!("{prefix}\n{body}\nreturn modLib.createMod"))
        .set_name("@original-ModTools-createMod")
        .eval()
        .unwrap()
}

#[test]
fn raw_original_create_mod_keeps_argument_slots_nil_holes_bytes_and_shared_values() {
    use native::*;
    use poe_optimizer_data::modifier_parser::*;
    let lua = source();
    let original = original_create_mod(&lua);
    let args = ParserProgramValueList {
        values: vec![],
        tail: Some(Box::new(ParserProgramPack::Varargs)),
    };
    let call = ParserProgramCall {
        binding: 0,
        receiver: None,
        arguments: args,
    };
    let plan = compile(
        0,
        true,
        vec![constructor()],
        vec![s(S::Return {
            values: ParserProgramValueList {
                values: vec![],
                tail: Some(Box::new(ParserProgramPack::Call { call })),
            },
        })],
    );
    for expression in [
        "return",
        "return nil,nil,nil",
        "return false,17,-0",
        "local t={marker=7};return t,'LIST',t,t,t,t",
        "local t={};t.self=t;return 'X','LIST',t,'source',32,64,t,nil,t",
        "return 'X','BASE',false,nil,32,64,nil,false,nil",
        "return 'raw\\0\\255',false,0/0,'\\0\\255',nil,false",
        "return 'X','BASE',1,'123',nil,2,{},nil",
    ] {
        let args: MultiValue = lua.load(expression).eval().unwrap();
        let incoming = from_source(&capture(args.clone()));
        let expected = capture(original.call(args).unwrap());
        let output = execute(&plan, &incoming).unwrap();
        assert_eq!(graph(output.graph()), expected, "{expression}");
    }
}

#[test]
fn raw_lazy_values_and_source_error_order_match_interpreter() {
    use native::*;
    use poe_optimizer_engine::parser_program::{ProgramRuntimeErrorKind, ProgramValue as V};
    let lua = source();
    let plan = compile(
        2,
        false,
        vec![],
        vec![ret(vec![
            binary(B::Or, l(0), unary(U::Negate, l(1))),
            binary(B::And, l(0), l(1)),
        ])],
    );
    let f: Function = lua
        .load("return function(a,b) return a or -b,a and b end")
        .eval()
        .unwrap();
    for (a, b) in [
        (Value::Boolean(true), Value::Boolean(false)),
        (Value::Number(0.), Value::Boolean(false)),
        (Value::Boolean(false), Value::Number(3.)),
        (Value::Nil, Value::Number(-0.)),
    ] {
        let args = MultiValue::from_vec(vec![a, b]);
        let incoming = from_source(&capture(args.clone()));
        assert_eq!(
            graph(execute(&plan, &incoming).unwrap().graph()),
            capture(f.call(args).unwrap())
        );
    }
    let plan = compile(
        0,
        false,
        vec![],
        vec![
            declare(0, table(vec![])),
            set(l(0), nil(), unary(U::Negate, boolean(false))),
            ret(vec![l(0)]),
        ],
    );
    let error = execute(&plan, &Default::default()).unwrap_err();
    assert_eq!(error.kind, ProgramRuntimeErrorKind::Source);
    assert!(
        error.message.contains("arithmetic") || error.message.contains("number"),
        "{error}"
    );
    let error = lua
        .load("return {[nil]=-false}")
        .eval::<MultiValue>()
        .unwrap_err()
        .to_string();
    assert!(error.contains("arithmetic"), "{error}");
    // Method indexing precedes a later argument's arithmetic failure.
    let mut call = call(0, vec![unary(U::Negate, boolean(false))]);
    call.receiver = Some(Box::new(l(0)));
    let plan = compile(
        1,
        false,
        vec![binding(I::StringGsub)],
        vec![ret(vec![e(E::Call {
            call: Box::new(call),
        })])],
    );
    let error = execute(&plan, &input(vec![V::Number(12.)])).unwrap_err();
    assert_eq!(error.kind, ProgramRuntimeErrorKind::Source);
    assert!(
        error.message.contains("index") || error.message.contains("method"),
        "{error}"
    );
    let f: Function = lua
        .load("return function(v) return v:gsub(-false) end")
        .eval()
        .unwrap();
    assert!(
        f.call::<MultiValue>(12.)
            .unwrap_err()
            .to_string()
            .contains("index")
    );
}

#[test]
fn raw_method_resolution_does_not_call_or_reject_target_before_arguments() {
    use native::*;
    use poe_optimizer_engine::parser_program::ProgramRuntimeErrorKind;
    let lua = source();
    let mut call = call(0, vec![unary(U::Negate, boolean(false))]);
    call.receiver = Some(Box::new(l(0)));
    let plan = compile(
        1,
        false,
        vec![binding(I::StringGsub)],
        vec![ret(vec![e(E::Call {
            call: Box::new(call),
        })])],
    );
    let f: Function = lua
        .load("return function(t) return t:gsub(-false) end")
        .eval()
        .unwrap();
    for expression in ["return {}", "return {gsub=false}", "return {gsub={}}"] {
        let args: MultiValue = lua.load(expression).eval().unwrap();
        let incoming = from_source(&capture(args.clone()));
        let original = f.call::<MultiValue>(args).unwrap_err().to_string();
        assert!(original.contains("arithmetic"), "{original}");
        let error = execute(&plan, &incoming).unwrap_err();
        assert_eq!(
            error.kind,
            ProgramRuntimeErrorKind::Source,
            "{expression}: {error}"
        );
        assert!(
            error.message.contains("arithmetic") || error.message.contains("number"),
            "{expression}: {error}"
        );
    }
    let (ok, calls, events, err): (bool, i64, Table, String) = lua
        .load(
            r#"
      local calls,events=0,{}
      local target={gsub=function() calls=calls+1 end}
      local function argument() events[#events+1]='argument';error('argument failure') end
      local ok,err=pcall(function() return target:gsub(argument()) end)
      return ok,calls,events,err
    "#,
        )
        .eval()
        .unwrap();
    assert!(!ok);
    assert_eq!(calls, 0);
    assert_eq!(sequence(events), vec![bytes(b"argument")]);
    assert!(err.ends_with("argument failure"));

    // The selected method is an actual original constructor, represented by an
    // opaque catalog callback on the native side. Neither side reaches the body
    // when the argument fails; this does not compare function closure graphs.
    let target = lua.create_table().unwrap();
    target.set("gsub", original_create_mod(&lua)).unwrap();
    assert!(
        f.call::<MultiValue>(target.clone())
            .unwrap_err()
            .to_string()
            .contains("arithmetic")
    );
    let incoming = opaque_method_input();
    let error = execute(&plan, &incoming).unwrap_err();
    assert_eq!(error.kind, ProgramRuntimeErrorKind::Source);
    assert!(
        error.message.contains("arithmetic") || error.message.contains("number"),
        "{error}"
    );
    // With successful arguments, original Lua can call the resolved helper.
    // Native execution explicitly defers this unadmitted dynamic method target.
    let mut selected = native::call(0, vec![]);
    selected.receiver = Some(Box::new(l(0)));
    let deferred = compile(
        1,
        false,
        vec![binding(I::StringGsub)],
        vec![ret(vec![e(E::Call {
            call: Box::new(selected),
        })])],
    );
    assert_eq!(
        execute(&deferred, &incoming).unwrap_err().kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    let successful: Function = lua
        .load("return function(t) return t:gsub() end")
        .eval()
        .unwrap();
    assert!(successful.call::<Table>(target).is_ok());
}

#[test]
fn raw_positive_nan_step_is_observed_inside_lua_before_native_pairing() {
    use native::*;
    use poe_optimizer_engine::parser_program::ProgramValue as V;
    let plan = compile(
        3,
        false,
        vec![],
        vec![
            declare(3, table(vec![])),
            s(S::ForNumeric {
                local: 4,
                start: l(0),
                limit: l(1),
                step: l(2),
                body: vec![
                    append(l(3), l(4)),
                    branch(
                        binary(B::Equal, unary(U::Length, l(3)), n(4.)),
                        vec![s(S::Break)],
                    ),
                ],
            }),
            ret(vec![l(3)]),
        ],
    );
    let lua = source();
    let f:Function=lua.load("return function(a,b) local zero=0;local step=-(zero/zero);local t={};for i=a,b,step do t[#t+1]=i;if #t==4 then break end end;return step,t end").eval().unwrap();
    for (a, b) in [(1., 2.), (2., 1.)] {
        let mut result: MultiValue = f.call((a, b)).unwrap();
        let Value::Number(step) = result.pop_front().unwrap() else {
            panic!("number echo")
        };
        assert_eq!(
            step.to_bits(),
            0x7ff8_0000_0000_0000,
            "source-generated positive NaN"
        );
        let original = capture(result);
        let native = execute(
            &plan,
            &input(vec![V::Number(a), V::Number(b), V::Number(step)]),
        )
        .unwrap();
        assert_eq!(graph(native.graph()), original);
    }
}
