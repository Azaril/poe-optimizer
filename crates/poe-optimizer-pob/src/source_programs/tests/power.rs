use super::*;
use crate::source_programs::capture::SourceClosureObserver;
use std::path::PathBuf;

fn shape(expression: &ParserProgramExpr) -> String {
    match &expression.operation {
        ParserProgramExprKind::Literal {
            value: ParserFactoryLiteral::Number(value),
        } => value.to_string(),
        ParserProgramExprKind::Local { local } => format!("v{local}"),
        ParserProgramExprKind::Unary { operation, value } => {
            format!("{operation:?}({})", shape(value))
        }
        ParserProgramExprKind::Binary {
            operation,
            left,
            right,
        } => format!("{operation:?}({},{})", shape(left), shape(right)),
        ParserProgramExprKind::SourceBinary {
            operation,
            left,
            right,
        } => {
            let left = match &**left {
                ParserProgramAssignmentOperand::LocalRegister { local } => format!("v{local}"),
                ParserProgramAssignmentOperand::Evaluated { value } => shape(value),
            };
            format!("{operation:?}({left},{})", shape(right))
        }
        ParserProgramExprKind::Call { call } => call_shape(call),
        other => panic!("unexpected expression: {other:?}"),
    }
}
fn call_shape(call: &ParserProgramCall) -> String {
    assert!(call.arguments.tail.is_none());
    let target = call
        .receiver
        .as_deref()
        .map(shape)
        .unwrap_or_else(|| format!("binding{}", call.binding));
    format!(
        "{target}({})",
        call.arguments
            .values
            .iter()
            .map(shape)
            .collect::<Vec<_>>()
            .join(",")
    )
}
fn returned_shape(statement: &ParserProgramStatement) -> String {
    let ParserProgramStatementKind::Return { values } = &statement.operation else {
        panic!("return")
    };
    if let Some(ParserProgramPack::Call { call }) = values.tail.as_deref() {
        assert!(values.values.is_empty());
        call_shape(call)
    } else {
        assert!(values.tail.is_none());
        assert_eq!(values.values.len(), 1);
        shape(&values.values[0])
    }
}
#[test]
fn standalone_power_preserves_lua_precedence_and_right_associative_operand_structure() {
    for (expression, expected) in [
        ("-2^2", "Negate(Power(2,2))"),
        ("(-2)^2", "Power(Negate(2),2)"),
        ("2^-2", "Power(2,Negate(2))"),
        ("(-2)^-3", "Power(Negate(2),Negate(3))"),
        ("2^3^2", "Power(2,Power(3,2))"),
        ("2^-2^2", "Power(2,Negate(Power(2,2)))"),
        ("-2^-2", "Negate(Power(2,Negate(2)))"),
        ("-2^2^3", "Negate(Power(2,Power(2,3)))"),
        ("2*3^2+1", "Add(Multiply(2,Power(3,2)),1)"),
        ("2^3*4", "Multiply(Power(2,3),4)"),
        ("a()^b()^c()", "Power(v0(),Power(v1(),v2()))"),
    ] {
        let text = format!("function(a,b,c) return {expression} end\n");
        let owner =
            SourceProgramOwner::new(definitions(&text, vec![callback(&text, 1, 1, vec![])]))
                .unwrap();
        let lowered = lower_from_sources(&sources(&text), &owner).unwrap();
        assert!(
            lowered.unsupported().is_empty(),
            "{expression}: {:?}",
            lowered.unsupported()
        );
        let program = &lowered.catalog().data().programs[0];
        assert_eq!(returned_shape(&program.body[0]), expected, "{expression}");
    }
}
#[test]
fn original_common_round_retains_both_decimal_powers_and_actual_floor_capture() {
    const COMMON: &str = "src/Modules/Common.lua";
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
    let common = crate::source::read_verified_text(&root, COMMON).unwrap();
    let start = common.find("function round(val, dec)\n").unwrap();
    let end = start + common[start..].find("\nend\n").unwrap() + 5;
    let prefix = common[..start]
        .split_inclusive('\n')
        .map(|line| {
            if line.starts_with("local m_floor = math.floor") {
                line.to_string()
            } else {
                "\n".into()
            }
        })
        .collect::<String>();
    let lua = Lua::new();
    let observer = SourceClosureObserver::capture_before_source(&lua).unwrap();
    // Execute the unchanged captured local and entire original function, with
    // exact file/line identities; full runtime call parity belongs to integration.
    lua.load(format!("{prefix}{}", &common[start..end]))
        .set_name(format!("@{COMMON}"))
        .exec()
        .unwrap();
    let function = lua.globals().raw_get("round").unwrap();
    let source = ItemLoadingSource {
        upstream_revision: "a".repeat(40),
        files: [(COMMON.into(), hash(common.as_bytes()))].into(),
        construction_spans: BTreeMap::new(),
        module_order: vec![COMMON.into()],
    };
    let sources = [(COMMON.into(), common)].into();
    let observed = observer
        .observe(&lua, &sources, source, &[("round".into(), function)].into())
        .unwrap();
    let owner = SourceProgramOwner::new(observed.definitions().clone()).unwrap();
    let lowered = lower_from_sources(&sources, &owner).unwrap();
    assert!(
        lowered.unsupported().is_empty(),
        "{:?}",
        lowered.unsupported()
    );
    assert_eq!(lowered.catalog().data().programs.len(), 1);
    let program = &lowered.catalog().data().programs[0];
    assert_eq!(program.callback, observed.callbacks()["round"]);
    assert_eq!(program.parameter_count, 2);
    assert_eq!(program.body.len(), 1);
    let ParserProgramStatementKind::If {
        branches,
        otherwise,
    } = &program.body[0].operation
    else {
        panic!("if")
    };
    assert_eq!(branches.len(), 1);
    assert_eq!(shape(&branches[0].condition), "v1");
    assert_eq!(branches[0].body.len(), 1);
    assert_eq!(otherwise.len(), 1);
    assert_eq!(
        returned_shape(&branches[0].body[0]),
        "Divide(binding0(Add(Multiply(v0,Power(10,v1)),0.5)),Power(10,v1))"
    );
    assert_eq!(returned_shape(&otherwise[0]), "binding0(Add(v0,0.5))");
    assert!(matches!(
        program.bindings.as_slice(),
        [ParserProgramBinding::Intrinsic {
            operation: ParserProgramIntrinsic::MathFloor,
            source: ParserProgramIntrinsicSource::Captured { upvalue: 0, .. }
        }]
    ));
    assert_eq!(program.provenance.source.line, 722);
    assert_eq!(program.provenance.source.end_line, 728);
}
#[test]
fn an_unexecuted_unsupported_decimal_branch_still_rejects_the_entire_function() {
    let text = "function(value, dec) if dec then return value % 10 else return math.floor(value) end end\n";
    let owner =
        SourceProgramOwner::new(definitions(text, vec![callback(text, 1, 1, vec![])])).unwrap();
    let lowered = lower_from_sources(&sources(text), &owner).unwrap();
    assert!(lowered.catalog().data().programs.is_empty());
    assert_eq!(lowered.unsupported().len(), 1);
}
