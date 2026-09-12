use super::*;
use crate::source_programs::capture::{
    SourceCaptureContext, SourceClosureObserver, SourceEnvironmentSelection, SourceTableSelection,
};
use poe_optimizer_engine::source_program::{
    CompiledSourcePrograms, ProgramLimits, ProgramValue, ProgramValueGraph,
};
use std::{collections::BTreeSet, path::PathBuf};
fn project(table: mlua::Table, fields: &[&str]) -> SourceTableSelection {
    SourceTableSelection {
        table,
        fields: fields.iter().map(|s| (*s).to_owned()).collect(),
        indexed: BTreeSet::new(),
        allow_index_fallback: false,
        allow_call_fallback: false,
    }
}
#[test]
fn original_global_bit_helper_bodies_lower_with_their_real_captures_and_environment() {
    const GLOBAL: &str = "src/Data/Global.lua";
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
    let text = crate::source::read_verified_text(&root, GLOBAL).unwrap();
    let lines = text.split_inclusive('\n').collect::<Vec<_>>();
    assert_eq!(lines[122], "local HIGH_MASK_53 = 0x1FFFFF\n");
    assert_eq!(lines[219], "end\n");
    let lua = Lua::new();
    let observer = SourceClosureObserver::capture_before_source(&lua).unwrap();
    // This bounded unit loads the unchanged complete helper family and actual
    // capture declarations at their exact original lines. Initialized full-PoB
    // graph observation and parser execution remain integration gates.
    lua.load(format!("{}{}", "\n".repeat(122), lines[122..220].concat()))
        .set_name(format!("@{GLOBAL}"))
        .exec()
        .unwrap();
    let roots = ["OR64", "AND64", "XOR64", "NOT64"]
        .into_iter()
        .map(|name| {
            (
                name.to_owned(),
                lua.globals().raw_get::<mlua::Function>(name).unwrap(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let source = ItemLoadingSource {
        upstream_revision: "a".repeat(40),
        files: [(GLOBAL.into(), hash(text.as_bytes()))].into(),
        construction_spans: BTreeMap::new(),
        module_order: vec![GLOBAL.into()],
    };
    let sources = [(GLOBAL.into(), text)].into();
    let context = SourceCaptureContext {
        environment: Some(SourceEnvironmentSelection {
            table: lua.globals(),
            root_name: "Globals".into(),
        }),
        projections: vec![
            project(lua.globals(), &["math", "bit"]),
            project(lua.globals().raw_get("math").unwrap(), &["floor"]),
            project(
                lua.globals().raw_get("bit").unwrap(),
                &["band", "bor", "bxor", "bnot"],
            ),
        ],
        ..SourceCaptureContext::default()
    };
    let observed = observer
        .observe_with_context(&lua, &sources, source, &roots, context)
        .unwrap();
    let lowered = lower_from_sources(&sources, observed.owner()).unwrap();
    assert!(
        lowered.unsupported().is_empty(),
        "{:?}",
        lowered.unsupported()
    );
    assert_eq!(lowered.catalog().data().programs.len(), 7);
    let mut source_ranges = lowered
        .catalog()
        .data()
        .programs
        .iter()
        .map(|p| (p.provenance.source.line, p.provenance.source.end_line))
        .collect::<Vec<_>>();
    source_ranges.sort_unstable();
    assert_eq!(
        source_ranges,
        [
            (129, 135),
            (137, 143),
            (145, 151),
            (153, 168),
            (170, 185),
            (187, 202),
            (204, 220)
        ]
    );
    for operation in [
        ParserProgramIntrinsic::MathFloor,
        ParserProgramIntrinsic::BitBand,
        ParserProgramIntrinsic::BitBor,
        ParserProgramIntrinsic::BitBxor,
        ParserProgramIntrinsic::BitBnot,
    ] {
        assert_eq!(
            observed
                .owner()
                .definitions()
                .unwrap()
                .intrinsics
                .values()
                .filter(|v| **v == operation)
                .count(),
            1
        );
    }
    let compiled = CompiledSourcePrograms::new(lowered.catalog()).unwrap();
    for name in ["OR64", "AND64", "XOR64", "NOT64"] {
        for args in [
            vec![0.0],
            vec![7.0, 3.0],
            vec![4294967296.0, 1.0],
            vec![7.0, 3.0, 1.0],
        ] {
            let original: f64 = roots[name]
                .call(mlua::MultiValue::from_vec(
                    args.iter().copied().map(mlua::Value::Number).collect(),
                ))
                .unwrap();
            let input = ProgramValueGraph {
                values: args.iter().copied().map(ProgramValue::Number).collect(),
                tables: vec![],
            };
            let output = compiled
                .execute(observed.callbacks()[name], &input, ProgramLimits::default())
                .unwrap();
            assert_eq!(
                output.graph().values,
                [ProgramValue::Number(original)],
                "{name} {args:?}"
            );
        }
    }
}
