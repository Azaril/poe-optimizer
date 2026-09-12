//! Shared actual-parser observation/lowering. No native compilation or import here.
use super::classes::{self, Primitives};
use mlua::{Function, Lua, Table, Value};
use poe_optimizer_engine::source_program::ProgramValue;
use poe_optimizer_pob::source_programs::{
    SourceProgramExtraction, capture::*, lower_observed_closures_and_constructors_from_sources,
    lower_observed_from_sources,
};
use serde_json::{Value as Json, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

pub struct CapturedParser {
    pub observed: ObservedSourceSession,
    pub lowered: SourceProgramExtraction,
    pub inventory: Json,
}
pub const PATH: &str = "tests/support/source_program_parser_scan.lua";
pub const TEXT: &str = include_str!("source_program_parser_scan.lua");
pub const PROBES: &[&str] = &["same", "lookup", "replace", "mutate", "distinct"];
// These are the dictionary arguments of actual scan calls in the complete inner body.
pub const DICTIONARIES: &[&str] = &[
    "specialModList",
    "preFlagList",
    "preSkillNameList",
    "formList",
    "modTagList",
    "skillNameList",
    "penTypes",
    "modNameList",
    "baseCostTypes",
    "costTypes",
    "flagTypes",
    "modFlagList",
    "suffixTypes",
];
#[allow(clippy::too_many_arguments)]
pub fn capture(
    lua: &Lua,
    primitives: &Primitives,
    scan: &Function,
    parser: &Function,
    probes: &Table,
    dictionaries: &BTreeMap<String, Table>,
    with_closures: bool,
    extra_callbacks: BTreeMap<String, Function>,
) -> CapturedParser {
    let vendor =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
    let mut texts = BTreeMap::from([(PATH.into(), TEXT.into())]);
    for path in [
        "src/Modules/Common.lua",
        "src/Modules/ModTools.lua",
        "src/Modules/ModParser.lua",
        "src/Data/Global.lua",
    ] {
        texts.insert(
            path.into(),
            poe_optimizer_pob::source::read_verified_text(&vendor, path).unwrap(),
        );
    }
    let (provenance, source_names) = classes::inventory(lua, &vendor, &texts);
    let mut callbacks = BTreeMap::from([
        ("original.scan".into(), scan.clone()),
        ("original.parser".into(), parser.clone()),
    ]);
    for name in PROBES {
        callbacks.insert(format!("probe.{name}"), probes.raw_get(*name).unwrap());
    }
    let globals = lua.globals();
    for (name, function) in extra_callbacks {
        assert!(
            callbacks.insert(name, function).is_none(),
            "duplicate callback root"
        );
    }
    let mut projections = vec![SourceTableSelection {
        table: globals.clone(),
        // Exact original global dependencies; legacy observation stays unchanged.
        fields: if with_closures {
            [
                "copyTable",
                "foo",
                "type",
                "unpack",
                "tonumber",
                "math",
                "bit",
            ]
            .map(str::to_owned)
            .into()
        } else {
            ["copyTable", "foo"].map(str::to_owned).into()
        },
        indexed: BTreeSet::new(),
        allow_index_fallback: false,
        allow_call_fallback: false,
    }];
    if with_closures {
        for (name, fields) in [
            ("math", vec!["floor"]),
            ("bit", vec!["band", "bor", "bxor", "bnot"]),
        ] {
            projections.push(SourceTableSelection {
                table: globals.raw_get(name).unwrap(),
                fields: fields.into_iter().map(str::to_owned).collect(),
                indexed: BTreeSet::new(),
                allow_index_fallback: false,
                allow_call_fallback: false,
            });
        }
    }
    let cache: Table = globals
        .raw_get::<Table>("modLib")
        .unwrap()
        .raw_get("parseModCache")
        .unwrap();
    assert_eq!(
        primitives.captured_value(parser, "cache"),
        Value::Table(cache.clone())
    );
    let mut state_roots: BTreeMap<_, _> = dictionaries
        .iter()
        .map(|(name, table)| (format!("dictionary.{name}"), Value::Table(table.clone())))
        .collect();
    state_roots.insert("public.cache".into(), Value::Table(cache));
    let observed = primitives
        .observer
        .observe_session(
            lua,
            &texts,
            provenance,
            SourceSessionCaptureRequest {
                callbacks,
                state_roots,
                definitions: SourceCaptureContext {
                    capture_iteration: true,
                    environment: Some(SourceEnvironmentSelection {
                        table: globals.clone(),
                        root_name: "Environment".into(),
                    }),
                    projections,
                    source_names,
                },
                ..SourceSessionCaptureRequest::default()
            },
        )
        .unwrap();
    let input = observed.input();
    let ProgramValue::Closure(id) =
        input.state.values[observed.root_index("original.parser").unwrap()]
    else {
        panic!("actual parser closure")
    };
    let closure = &input.closures[id.0 as usize - 1];
    let prototype = observed
        .owner()
        .resolve_closure_prototype(&closure.prototype)
        .unwrap();
    let callback = observed.owner().callback(prototype.callback).unwrap();
    let slot = callback
        .upvalues
        .iter()
        .position(|value| value.name == "cache")
        .unwrap();
    assert_eq!(
        input.cells[closure.captures[slot].0 as usize - 1],
        input.state.values[observed.root_index("public.cache").unwrap()],
        "published cache and closure cell preserve the same identity"
    );
    let lowered = if with_closures {
        lower_observed_closures_and_constructors_from_sources(
            &texts,
            observed.owner(),
            observed.closure_observations().unwrap(),
            observed.constructor_observations().unwrap(),
        )
    } else {
        lower_observed_from_sources(
            &texts,
            observed.owner(),
            observed.constructor_observations().unwrap(),
        )
    }
    .unwrap();
    // Preserve the exact declarations behind each frontier so the next source
    // dependency is reviewable without guessing from owner-local callback IDs.
    let unsupported_sources: Vec<_> = lowered
        .unsupported()
        .iter()
        .map(|(callback, reason)| {
            json!({"callback":callback,
                "declaration":observed.owner().callback(*callback).unwrap().kind,
                "reason":reason})
        })
        .collect();
    let creation_sources: Vec<_> = lowered
        .catalog()
        .closure_creations()
        .into_iter()
        .flat_map(|facet| &facet.sites)
        .map(|site| {
            json!({"callback":site.callback,
            "declaration":observed.owner().callback(site.callback).unwrap().kind,
            "expression":site.expression,"child":site.child_provenance,
            "capture_count":site.captures.len()})
        })
        .collect();
    let report = json!({"closure_creation_observation":with_closures,"compiled_programs":lowered.catalog().data().programs.len(),"creation_sources":creation_sources,"creation_sites":lowered.catalog().closure_creations().map(|c|c.sites.len()).unwrap_or(0),"unsupported_bodies":lowered.unsupported(),
        "unsupported_sources":unsupported_sources,
        "constructor_frontiers":lowered.constructor_unsupported(),
        "published_cache_alias_retained":true,"state_tables":observed.input().state.tables.len(), "closures":observed.input().closures.len(),
        "capture_cells":observed.input().cells.len(), "source":observed.owner().source()});
    CapturedParser {
        observed,
        lowered,
        inventory: report,
    }
}
