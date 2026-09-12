//! Complete original ModParser.scan over the initialized public parser's live graph.
//! This dependency gate does not claim complete parsing or numerical build parity.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/source_program_classes.rs"]
mod classes;
#[path = "support/source_program_observation.rs"]
mod observation;
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
#[path = "support/source_program_warm.rs"]
mod warm;
use classes::Primitives;
use mlua::{Function, Lua, MultiValue, Table, Value};
use poe_optimizer_engine::{lua_pattern::MatchLimits, source_program::*};
use poe_optimizer_pob::source_programs::{
    capture::*, lower_observed_closures_and_constructors_from_sources, lower_observed_from_sources,
};
use serde_json::{Value as Json, json};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    rc::Rc,
    time::{Duration, Instant},
};
const PATH: &str = "tests/support/source_program_parser_scan.lua";
const TEXT: &str = include_str!("support/source_program_parser_scan.lua");
const PROBES: &[&str] = &["same", "lookup", "replace", "mutate", "distinct"];
// These are the dictionary arguments of actual scan calls in the complete inner body.
const DICTIONARIES: &[&str] = &[
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
fn limits() -> ProgramLimits {
    ProgramLimits {
        max_steps: 50_000_000,
        max_values: 8_000_000,
        max_bytes: 256 * 1024 * 1024,
        max_tables: 100_000,
        pattern: MatchLimits {
            max_steps: 200_000_000,
            ..MatchLimits::default()
        },
        ..ProgramLimits::default()
    }
}
struct Pair {
    observed: ObservedSourceSession,
    compiled: CompiledSourcePrograms,
    session: ProgramSession,
    roots: Vec<SessionValue>,
}
impl Pair {
    fn root(&self, name: &str) -> SessionValue {
        self.roots[self.observed.root_index(name).unwrap()].clone()
    }
    fn call(
        &mut self,
        name: &str,
        values: &[SessionValue],
    ) -> Result<Vec<SessionValue>, ProgramRuntimeError> {
        self.session.invoke_callable(&self.root(name), values)
    }
    fn args(&mut self, values: &[Value]) -> Vec<SessionValue> {
        self.session.borrow(&observation::capture(values)).unwrap()
    }
    fn plain(&mut self, values: &[SessionValue]) -> Json {
        observation::canonical(self.session.snapshot(values).unwrap().graph())
    }
    fn boolean(&mut self, values: &[SessionValue]) -> bool {
        let graph = self.session.snapshot(values).unwrap();
        assert_eq!(graph.graph().values.len(), 1);
        let ProgramValue::Boolean(value) = graph.graph().values[0] else {
            panic!("boolean result")
        };
        value
    }
}
fn observe(
    lua: &Lua,
    primitives: &Primitives,
    scan: &Function,
    parser: &Function,
    probes: &Table,
    dictionaries: &BTreeMap<String, Table>,
    with_closures: bool,
) -> (Pair, Json) {
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
                    projections: vec![SourceTableSelection {
                        table: globals,
                        // The complete parser and wrapper read these original globals.
                        // Keep the preceding scanner-only observation unchanged.
                        fields: if with_closures {
                            ["copyTable", "foo", "type", "unpack"]
                                .map(str::to_owned)
                                .into()
                        } else {
                            ["copyTable", "foo"].map(str::to_owned).into()
                        },
                        indexed: BTreeSet::new(),
                        allow_index_fallback: false,
                        allow_call_fallback: false,
                    }],
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
    let compiled = CompiledSourcePrograms::new(lowered.catalog()).unwrap();
    let (session, roots) = compiled
        .session_from_input(observed.input(), limits())
        .unwrap();
    (
        Pair {
            observed,
            compiled,
            session,
            roots,
        },
        report,
    )
}
fn compare_results(
    pair: &mut Pair,
    dictionary: &Table,
    name: &str,
    actual: &[Value],
    native: &[SessionValue],
) -> Json {
    assert_eq!(actual.len(), native.len(), "scan result cardinality {name}");
    assert!(matches!(actual.len(), 2 | 3));
    let details = pair.plain(&native[1..]);
    assert_eq!(
        details,
        observation::canonical(&observation::capture(&actual[1..])),
        "scan remainder/captures {name}"
    );
    let selected = if actual[0] == Value::Nil {
        assert_eq!(
            pair.plain(&native[..1]),
            observation::canonical(&observation::capture(&actual[..1]))
        );
        None
    } else {
        let key = dictionary
            .clone()
            .pairs::<mlua::LuaString, Value>()
            .map(Result::unwrap)
            .find_map(|(key, value)| (value == actual[0]).then_some(key))
            .expect("returned exact dictionary value");
        let key_value = pair.args(&[Value::String(key.clone())]).remove(0);
        let checked = pair
            .call(
                "probe.same",
                &[
                    native[0].clone(),
                    pair.root(&format!("dictionary.{name}")),
                    key_value,
                ],
            )
            .unwrap();
        assert!(
            pair.boolean(&checked),
            "selected dictionary value identity {name}"
        );
        Some(String::from_utf8_lossy(&key.as_bytes()).into_owned())
    };
    json!({"selected":selected,"result_count":actual.len(),"remainder_and_captures":details})
}
fn scan_case(
    lua: &Lua,
    scan: &Function,
    pair: &mut Pair,
    dictionary: &Table,
    name: &str,
    line: &[u8],
    plain: bool,
) -> (Json, Vec<Value>, Vec<SessionValue>) {
    let text = Value::String(lua.create_string(line).unwrap());
    let actual = scan
        .call::<MultiValue>((text.clone(), dictionary.clone(), plain))
        .unwrap()
        .into_vec();
    let mut args = pair.args(&[text, Value::Boolean(plain)]);
    args.insert(1, pair.root(&format!("dictionary.{name}")));
    let native = pair
        .call("original.scan", &args)
        .unwrap_or_else(|e| panic!("scan {name} {line:?}: {e}"));
    let mut row = compare_results(pair, dictionary, name, &actual, &native);
    row["line_bytes"] = json!(line);
    row["plain"] = json!(plain);
    (row, actual, native)
}
fn lines(xml: &str) -> Vec<Vec<u8>> {
    let document = roxmltree::Document::parse(xml).unwrap();
    let all = document
        .descendants()
        .filter(|n| n.has_tag_name("Item"))
        .filter_map(|n| n.text())
        .flat_map(str::lines)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.as_bytes().to_vec())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    assert!(all.len() >= 12, "real build supplies item text");
    (0..12)
        .map(|i| all[i * (all.len() - 1) / 11].clone())
        .collect()
}
fn run(lua: &Lua, primitives: &Primitives, xml: &str, with_closures: bool) -> Json {
    let jit: Table = lua.globals().raw_get("jit").unwrap();
    jit.raw_get::<Function>("off")
        .unwrap()
        .call::<()>(())
        .unwrap();
    jit.raw_get::<Function>("flush")
        .unwrap()
        .call::<()>(())
        .unwrap();
    let parser: Function = lua
        .globals()
        .raw_get::<Table>("modLib")
        .unwrap()
        .raw_get("parseMod")
        .unwrap();
    let inner = primitives
        .captured_value(&parser, "parseMod")
        .as_function()
        .unwrap()
        .clone();
    let scan = primitives
        .captured_value(&inner, "scan")
        .as_function()
        .unwrap()
        .clone();
    assert!(
        scan.info()
            .source
            .as_deref()
            .unwrap()
            .ends_with("Modules/ModParser.lua")
    );
    let probes: Table = lua.load(TEXT).set_name(format!("@{PATH}")).eval().unwrap();
    let mut dictionaries = DICTIONARIES
        .iter()
        .map(|name| {
            (
                (*name).to_owned(),
                primitives
                    .captured_value(&inner, name)
                    .as_table()
                    .unwrap()
                    .clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    // Explicit unit adversaries complement actual tables and input text. The algorithm
    // being tested is still the complete original function, never a Rust scan recipe.
    let fixtures: Table = lua
        .load(
            r#"return {
        ['ab']={id=1}, ['a(b)']={id=2}, ['(a)b']={id=3},
        ['(%d+)%%']={id=4}, ['x()']={id=5}, ['z']=false,
        ['']={id=6}, ['%z']={id=7}, ['%b()']={id=8},
        ['%f[%a](%a+)']={id=9}, ['()()()()()()']={id=10}
    }"#,
        )
        .eval()
        .unwrap();
    dictionaries.insert("fixture".into(), fixtures);
    let malformed = lua.create_table().unwrap();
    malformed.raw_set("[", true).unwrap();
    dictionaries.insert("malformed".into(), malformed);

    let (mut pair, inventory) = observe(
        lua,
        primitives,
        &scan,
        &parser,
        &probes,
        &dictionaries,
        with_closures,
    );
    let mut rows = Vec::new();
    for name in DICTIONARIES {
        let table = &dictionaries[*name];
        for line in lines(xml) {
            // Actual callers use plain for these literal-name dictionaries.
            let plain = matches!(
                *name,
                "penTypes" | "modNameList" | "baseCostTypes" | "costTypes" | "modFlagList"
            );
            let (mut row, _, _) = scan_case(lua, &scan, &mut pair, table, name, &line, plain);
            row["dictionary"] = json!(name);
            rows.push(row);
        }
    }
    let mut fixture_rows = Vec::new();
    for line in [
        b"AB".as_slice(),
        b"12% AB",
        b"x",
        b"z",
        b"",
        b"A\0B",
        b"(abc)",
        b"letters",
        &[255, b'A'],
    ] {
        for plain in [false, true] {
            let (row, _, _) = scan_case(
                lua,
                &scan,
                &mut pair,
                &dictionaries["fixture"],
                "fixture",
                line,
                plain,
            );
            fixture_rows.push(row);
        }
    }
    let mut source_errors = Vec::new();
    for (name, subject, invalid_dictionary) in [
        ("nil_subject", Value::Nil, true),
        ("boolean_subject", Value::Boolean(false), true),
        ("numeric_subject", Value::Integer(7), true),
        (
            "nil_dictionary",
            Value::String(lua.create_string("abc").unwrap()),
            true,
        ),
        (
            "malformed_pattern",
            Value::String(lua.create_string("abc").unwrap()),
            false,
        ),
    ] {
        let dictionary = if invalid_dictionary {
            Value::Nil
        } else {
            Value::Table(dictionaries["malformed"].clone())
        };
        let actual = scan
            .call::<MultiValue>((subject.clone(), dictionary, false))
            .unwrap_err();
        let mut args = pair.args(&[subject, Value::Boolean(false)]);
        let dictionary = if invalid_dictionary {
            pair.args(&[Value::Nil]).remove(0)
        } else {
            pair.root("dictionary.malformed")
        };
        args.insert(1, dictionary);
        let before = (
            pair.session.steps(),
            pair.session.pattern_steps(),
            pair.session.allocations(),
        );
        let native = pair.call("original.scan", &args).unwrap_err();
        assert_eq!(
            native.kind,
            ProgramRuntimeErrorKind::Source,
            "{name}: {actual}; {native}"
        );
        assert!(pair.session.steps() >= before.0 && pair.session.pattern_steps() >= before.1);
        assert!(
            pair.session.allocations().values >= before.2.values
                && pair.session.allocations().bytes >= before.2.bytes
        );
        source_errors.push(json!({"case":name,"source":actual.to_string(),"native":native.message,"cumulative_charges_retained":true}));
    }
    // The same malformed pattern is literal data when plain mode is truthy.
    let (plain_malformed, _, _) = scan_case(
        lua,
        &scan,
        &mut pair,
        &dictionaries["malformed"],
        "malformed",
        b"a[b",
        true,
    );
    assert!(plain_malformed["selected"].is_string());
    let (baseline, actual, native) = scan_case(
        lua,
        &scan,
        &mut pair,
        &dictionaries["modNameList"],
        "modNameList",
        b"strength",
        true,
    );
    assert!(
        baseline["selected"].is_string(),
        "actual literal dictionary selects strength"
    );
    let key = lua
        .create_string(baseline["selected"].as_str().unwrap())
        .unwrap();
    let key_arg = pair.args(&[Value::String(key.clone())]).remove(0);
    let selected = pair
        .call(
            "probe.lookup",
            &[pair.root("dictionary.modNameList"), key_arg.clone()],
        )
        .unwrap();
    let false_arg = pair.args(&[Value::Boolean(false)]).remove(0);
    pair.call(
        "probe.replace",
        &[
            pair.root("dictionary.modNameList"),
            key_arg.clone(),
            false_arg,
        ],
    )
    .unwrap();
    let old: Value = dictionaries["modNameList"].raw_get(key.clone()).unwrap();
    dictionaries["modNameList"]
        .raw_set(key.clone(), false)
        .unwrap();
    let (changed, _, _) = scan_case(
        lua,
        &scan,
        &mut pair,
        &dictionaries["modNameList"],
        "modNameList",
        b"strength",
        true,
    );
    assert_ne!(changed, baseline);
    dictionaries["modNameList"]
        .raw_set(key.clone(), old)
        .unwrap();
    // The first native dictionary is still modified while a second session imports
    // the original input. Only the independent Lua reference has been restored.
    let (isolated, roots) = pair
        .compiled
        .session_from_input(pair.observed.input(), limits())
        .unwrap();
    let mut independent = Pair {
        observed: pair.observed.clone(),
        compiled: pair.compiled.clone(),
        session: isolated,
        roots,
    };
    let (independent_result, _, _) = scan_case(
        lua,
        &scan,
        &mut independent,
        &dictionaries["modNameList"],
        "modNameList",
        b"strength",
        true,
    );
    assert_eq!(independent_result, baseline);
    pair.call(
        "probe.replace",
        &[
            pair.root("dictionary.modNameList"),
            key_arg,
            selected[0].clone(),
        ],
    )
    .unwrap();
    let (restored, _, fresh) = scan_case(
        lua,
        &scan,
        &mut pair,
        &dictionaries["modNameList"],
        "modNameList",
        b"strength",
        true,
    );
    assert_eq!(restored, baseline);
    assert_ne!(actual[2], Value::Nil);
    let distinct = pair
        .call("probe.distinct", &[native[2].clone(), fresh[2].clone()])
        .unwrap();
    assert!(
        pair.boolean(&distinct),
        "each scan allocates a fresh captures table"
    );
    let changed_value = pair.args(&[Value::Integer(123)]).remove(0);
    let edited = pair
        .call("probe.mutate", &[native[2].clone(), changed_value])
        .unwrap();
    actual[2].as_table().unwrap().raw_set(1, 123).unwrap();
    assert_eq!(
        pair.plain(&edited),
        observation::canonical(&observation::capture(&actual[2..]))
    );
    assert_eq!(pair.plain(&fresh[1..]), baseline["remainder_and_captures"]);
    let driver = warm::SourceWarmDriver::new(lua).unwrap();
    let mut warmed = Vec::new();
    // LuaJIT cannot record pattern-mode string.find. The pattern comparisons
    // above retain those real pattern paths; these warm vectors exercise literal
    // lookup in the same original scan and dictionaries without claiming pattern JIT.
    for (name, line, plain) in [
        ("formList", "+15 to maximum Life", true),
        ("modNameList", "strength", true),
        ("fixture", "12% AB", true),
    ] {
        let args = [
            Value::String(lua.create_string(line).unwrap()),
            Value::Table(dictionaries[name].clone()),
            Value::Boolean(plain),
        ];
        let direct = driver
            .run(lua, &scan, &args, None)
            .unwrap_or_else(|error| panic!("direct warm {name}: {error}"));
        assert!(direct.success);
        let expected = scan
            .call::<MultiValue>(MultiValue::from_vec(args.to_vec()))
            .unwrap()
            .into_vec();
        assert_eq!(direct.value, expected[0]);
        let wrapper: Function = probes.raw_get("packed_scan").unwrap();
        let mut packed_args = vec![Value::Function(scan.clone())];
        packed_args.extend(args.clone());
        let packed = driver
            .run_with_target(lua, &wrapper, &scan, &packed_args, None)
            .unwrap_or_else(|error| panic!("packed warm {name}: {error}"));
        assert!(packed.success);
        let result = packed.value.as_table().unwrap();
        assert!(result.raw_get::<bool>("verified").unwrap());
        let count: usize = result.raw_get("n").unwrap();
        let actual = (1..=count)
            .map(|i| result.raw_get::<Value>(i).unwrap())
            .collect::<Vec<_>>();
        let (_, _, native) = scan_case(
            lua,
            &scan,
            &mut pair,
            &dictionaries[name],
            name,
            line.as_bytes(),
            plain,
        );
        let comparison = compare_results(&mut pair, &dictionaries[name], name, &actual, &native);
        warmed.push(json!({"dictionary":name,"plain":plain,"direct_calls":direct.calls,"direct_seed_calls":direct.seed_calls,"direct_target_traces":direct.target_live_traces,
            "packed_calls":packed.calls,"packed_seed_calls":packed.seed_calls,"packed_target_traces":packed.target_live_traces,"comparison":comparison}));
    }
    // The genuine public wrapper/captures/cache are in this same owner/session.
    // Its uncached call must expose the next real dependency; it is not replaced.
    let args = pair.args(&[Value::String(
        lua.create_string("native parser readiness sentinel never matches")
            .unwrap(),
    )]);
    let error = pair
        .call("original.parser", &args)
        .expect_err("full parser milestone not complete yet; extend acceptance before admission");
    assert_eq!(error.kind, ProgramRuntimeErrorKind::UnsupportedCapability);
    let frontier_declaration = error
        .callback
        .and_then(|callback| pair.observed.owner().callback(callback))
        .map(|callback| &callback.kind);
    json!({"inventory":inventory,"real_dictionary_cases":rows,"fixture_cases":fixture_rows,
        "dictionary_mutation":{"baseline":baseline,"changed":changed,"restored":restored},
        "source_errors":source_errors,"plain_malformed_pattern":plain_malformed,"captures_writable_and_independent":true,"session_isolation":true,"warmed":warmed,
        "public_parser_frontier":{"declaration":frontier_declaration,"kind":format!("{:?}",error.kind),"message":error.message,"callback":error.callback,"location":error.location},
        "steps":pair.session.steps(),"pattern_steps":pair.session.pattern_steps(),
        "native_complete_builds":0,"complete_public_parser":false})
}
#[test]
fn complete_original_scan_uses_initialized_parser_dictionaries_on_all_five_builds() {
    check_all_builds(false);
}
#[test]
fn original_parser_factory_graph_preserves_all_five_scan_contracts() {
    check_all_builds(true);
}
fn check_all_builds(with_closures: bool) {
    let test_name = if with_closures {
        "original_parser_factory_graph_preserves_all_five_scan_contracts"
    } else {
        "complete_original_scan_uses_initialized_parser_dictionaries_on_all_five_builds"
    };
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let destination = project.join(if with_closures {
        "runs/r2p-parser-factories"
    } else {
        "runs/r2n-parser-scan"
    });
    fs::create_dir_all(&destination).unwrap();
    if let Ok(build) = std::env::var("POE_PARSER_SCAN_CHILD") {
        assert!(["01", "02", "03", "04", "05"].contains(&build.as_str()));
        let xml = fs::read_to_string(project.join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{build}.xml"
        )))
        .unwrap();
        let primitives = Rc::new(RefCell::new(None));
        let scratch = tempfile::tempdir().unwrap();
        let result = source::observe_with_hooks(
            &project.join("vendor/path-of-building-poe2"),
            scratch.path(),
            &xml,
            None,
            false,
            Some(&|lua| {
                primitives.replace(Some(if with_closures {
                    Primitives::before_source_with_closures(lua)?
                } else {
                    Primitives::before_source_with_constructors(lua)?
                }));
                Ok(())
            }),
            Some(&|lua| {
                Ok(run(
                    lua,
                    primitives.borrow().as_ref().unwrap(),
                    &xml,
                    with_closures,
                ))
            }),
        )
        .unwrap();
        fs::write(
            destination.join(format!("build-{build}.json")),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        return;
    }
    for build in ["01", "02", "03", "04", "05"] {
        let log = fs::File::create(destination.join(format!("build-{build}.log"))).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test_name, "--nocapture"])
            .env("POE_PARSER_SCAN_CHILD", build)
            .current_dir(project.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "parser scan build {build}; see {destination:?}"
                );
                break;
            }
            if start.elapsed() > Duration::from_secs(300) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("parser scan child timeout {build}");
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}
