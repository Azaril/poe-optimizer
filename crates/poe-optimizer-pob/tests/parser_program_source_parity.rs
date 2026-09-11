//! Complete retained original callbacks versus source-extracted raw programs.
//! These controls do not admit a callback to the public parser or a complete build.
#[allow(dead_code)]
#[path = "support/callback_factories_source.rs"]
mod factory_source;
#[allow(dead_code)]
#[path = "support/mod_parser_public_source.rs"]
mod public_source;
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;
#[path = "support/parser_program_source.rs"]
mod source;

use mlua::{MultiValue, Value};
use poe_optimizer_data::modifier_parser::*;
use public_source::Atom;
use source::{Source, Target};

#[test]
fn four_complete_original_functions_keep_source_binding_and_legacy_dispositions() {
    let plan = source::plan();
    let original = Source::new();
    for target in Target::ALL {
        let id = target.id(plan.catalog().owner());
        let (first, last) = target.lines();
        assert_eq!(
            original.function(target).info().line_defined,
            Some(first as usize)
        );
        assert_eq!(
            original.function(target).info().last_line_defined,
            Some(last as usize)
        );
        assert!(matches!(
            plan.catalog().owner().factory(id),
            Some(ParserFactoryDisposition::Unsupported { .. })
        ));
        assert!(!source::extraction().unsupported().contains_key(&id));
    }
    if let Some(path) = std::env::var_os("POE_TYPED_PROGRAM_ARTIFACT") {
        use sha2::{Digest, Sha256};
        let extraction = source::extraction();
        let proved = Target::ALL.map(|target| target.id(plan.catalog().owner()));
        let unproved = extraction
            .catalog()
            .data()
            .callbacks
            .keys()
            .filter(|id| !proved.contains(id))
            .copied()
            .collect::<Vec<_>>();
        let owner_sha256 = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(extraction.catalog().owner().data()).unwrap())
        );
        let artifact = serde_json::json!({
            "schema_version": 1,
            "scope": "Complete source-lowering inventory; four callbacks receive independent raw source parity in this target. Raw comparisons do not establish public parser or whole-build parity.",
            "implementation_sha256": extraction.implementation_sha256(),
            "owner_serialized_sha256": owner_sha256,
            "source": extraction.catalog().owner().data().source,
            "programs": extraction.catalog().data(),
            "unsupported": extraction.unsupported(),
            "independently_paired_callbacks": proved,
            "unproved_program_callbacks": unproved,
        });
        // Only an explicit caller path is written, without overwriting evidence.
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        serde_json::to_writer_pretty(file, &artifact).unwrap();
    }
    let count = original.lookup.clone().pairs::<Value, Value>().count();
    assert_eq!(
        count,
        plan.catalog()
            .owner()
            .dictionary(ParserDictionary::GemIdLookup)
            .fields
            .len()
    );
    assert_eq!(
        count, 961,
        "pinned construction inventory; not an admission list"
    );
    eprintln!(
        "G3 inventory: {} complete lowered programs; four original targets independently exercised",
        source::extraction().catalog().data().programs.len()
    );
}

#[test]
fn property_requirements_word_counts_and_raw_value_graphs_match() {
    let plan = source::plan();
    let source = Source::new();
    let types = [
        "",
        "callerword",
        "caller word",
        "caller-word's 12",
        "---",
        "é\0word",
    ];
    let requirements = [
        Some("intelligence"),
        Some("dexterity"),
        Some("strength"),
        Some("Intelligence"),
        Some(""),
        None,
    ];
    for kind in types {
        for requirement in requirements {
            let args = MultiValue::from_vec(vec![
                Value::Number(-0.0),
                Value::Boolean(false),
                source.text("quality"),
                source.text(kind),
                requirement.map_or(Value::Nil, |v| source.text(v)),
            ]);
            source.pair(
                &plan,
                Target::Property,
                args,
                &format!("words {kind:?} req {requirement:?}"),
            );
        }
    }
    for values in [
        "return 0/0, nil, 'quality', 'caller word', nil",
        "return math.huge, {}, false, 'callerword', 'strength'",
        "return -math.huge, false, nil, '---', false",
        "return nil, nil, nil, '', nil",
        "local t={x=-0.0};t.self=t; return t, nil, t, 'caller word', {}",
        r"return '\255\0n', nil, '\254\0p', '\253foo\0bar', 'dexterity'",
    ] {
        source.pair(&plan, Target::Property, source.args(values), values);
    }
}

#[test]
fn property_exact_load_minion_lookup_truthiness_and_lazy_branches_match() {
    let source = Source::new();
    let plan = source::injected(
        &source,
        &[
            ("caller exact", ParserValue::Text("InjectedId".into())),
            ("caller load", ParserValue::Boolean(false)),
            ("load caller load", ParserValue::Number(0.0)),
            ("caller minion", ParserValue::Nil),
            ("load caller minion", ParserValue::Boolean(false)),
            ("caller minion minion", ParserValue::Text(String::new())),
            ("caller false", ParserValue::Boolean(false)),
            ("load caller false", ParserValue::Boolean(false)),
            ("caller false minion", ParserValue::Boolean(false)),
        ],
    );
    for name in [
        "caller exact",
        "caller load",
        "caller minion",
        "caller false",
    ] {
        let graph = source.pair(
            &plan,
            Target::Property,
            MultiValue::from_vec(vec![
                Value::Number(3.0),
                Value::Nil,
                source.text("quality"),
                source.text(name),
                source.text("intelligence"),
            ]),
            name,
        );
        assert!(graph.tables.iter().flat_map(|v| v.iter()).any(|(k, v)| *k
            == Atom::Bytes(b"key".to_vec())
            && *v
                == Atom::Bytes(if name == "caller false" {
                    b"quality".to_vec()
                } else {
                    b"level".to_vec()
                })));
    }
    // Source-only control beyond the existing named-dictionary data boundary:
    // numeric exact hits suppress both concatenations and string method lookup.
    // The validated injected catalog rejects this shape; do not hide that limit.
    let mut data = plan.catalog().owner().data().clone();
    // These raw-definition probes do not carry reviewed package dispatch claims.
    data.programs.admissions.clear();
    let lookup = data.dictionaries[&ParserDictionary::GemIdLookup];
    data.tables[lookup.0 as usize - 1]
        .indexed
        .insert(7, ParserValue::Boolean(true));
    assert!(
        ModifierParserCatalog::new(data)
            .unwrap_err()
            .to_string()
            .contains("non-string keys")
    );
    source.lookup.raw_set(7, true).unwrap();
    let raw: MultiValue = source
        .property
        .call(source.args("return 2,nil,'quality',7,nil"))
        .unwrap();
    assert_eq!(raw.len(), 1);
    source.lookup.raw_set(7, Value::Nil).unwrap();
    for (args, error) in [
        ("return 2,nil,'quality',8,nil", "index"),
        ("return 2,nil,'quality',nil,nil", "concatenate"),
        ("return 2,nil,'quality',false,nil", "concatenate"),
    ] {
        source.source_error(&plan, Target::Property, source.args(args), error);
    }
}

#[test]
fn grant_global_substitution_numeric_conversion_and_zero_results_match() {
    let source = Source::new();
    let plan = source::injected(
        &source,
        &[
            ("caller", ParserValue::Text("CallerId".into())),
            ("callerful", ParserValue::Number(0.0)),
            ("caller false", ParserValue::Boolean(false)),
            ("caller empty", ParserValue::Text(String::new())),
            ("caller true", ParserValue::Boolean(true)),
        ],
    );
    let levels = [
        "return nil",
        "return false",
        "return -0.0",
        "return ' -0 '",
        "return '0x1p-3'",
        "return 'inf'",
        "return 'nan'",
        "return 'bad'",
        "return '1\0ignored'",
        "return {}",
        "return math.huge",
    ];
    let mut pairs = 0;
    for name in [
        "caller",
        "caller skill skill",
        "caller skillful",
        "caller empty",
        "caller true",
        "caller false",
        "missing skill",
        "CALLER",
    ] {
        for level in levels {
            let value = source.args(level).pop_front().unwrap();
            let graph = source.pair(
                &plan,
                Target::Grant,
                MultiValue::from_vec(vec![source.text(name), value, Value::Boolean(false)]),
                &format!("{name} {level}"),
            );
            assert_eq!(
                graph.roots.len(),
                usize::from(!["caller false", "missing skill", "CALLER"].contains(&name)),
                "raw result arity"
            );
            pairs += 1;
        }
    }
    for support in [
        Value::Nil,
        Value::Boolean(false),
        Value::Boolean(true),
        Value::Table(source.table()),
    ] {
        source.pair(
            &plan,
            Target::Grant,
            MultiValue::from_vec(vec![source.text("caller"), source.text("7"), support]),
            "noSupports is retained without coercion",
        );
    }
    eprintln!("grant: {pairs} substitution/conversion/zero-return pairs + four noSupports pairs");
}

#[test]
fn forwarding_callbacks_retain_original_argument_and_tail_result_protocols() {
    let source = Source::new();
    let plan = source::injected(&source, &[("caller", ParserValue::Text("CallerId".into()))]);
    for name in ["caller", "caller skill skill", "missing"] {
        source.pair(
            &plan,
            Target::Plain,
            MultiValue::from_vec(vec![Value::Table(source.table()), source.text(name)]),
            "plain raw protocol",
        );
        for level in [
            Value::Nil,
            Value::Boolean(false),
            Value::Number(-0.0),
            source.text("0x10"),
            source.text("nan"),
            Value::Table(source.table()),
        ] {
            let graph = source.pair(
                &plan,
                Target::Level,
                MultiValue::from_vec(vec![
                    level,
                    source.text("ignored raw capture"),
                    source.text(name),
                ]),
                "leveled raw protocol",
            );
            assert_eq!(graph.roots.len(), usize::from(name != "missing"));
        }
    }
    for target in [Target::Grant, Target::Plain, Target::Level] {
        source.source_error(&plan, target, MultiValue::new(), "index");
    }
}

#[test]
fn malformed_receivers_error_and_definition_table_aliases_remain_visible() {
    let source = Source::new();
    let plan = source::plan();
    for args in [
        "return nil,1,false",
        "return 7,1,false",
        "return false,1,false",
        "return {},1,false",
        "return {gsub=false},1,false",
        "return {gsub={}},1,false",
    ] {
        source.source_error(
            &plan,
            Target::Grant,
            source.args(args),
            if args.contains('{') { "call" } else { "index" },
        );
    }
    let mut data = plan.catalog().owner().data().clone();
    // These raw-definition probes do not carry reviewed package dispatch claims.
    data.programs.admissions.clear();
    let lookup = data.dictionaries[&ParserDictionary::GemIdLookup];
    let table_id = ParserTableId(data.tables.len() as u32 + 1);
    data.tables.push(ParserTable {
        fields: std::collections::BTreeMap::from([
            ("payload".into(), ParserValue::Number(-0.0)),
            ("self".into(), ParserValue::Table(table_id)),
        ]),
        indexed: Default::default(),
    });
    data.tables[lookup.0 as usize - 1]
        .fields
        .insert("caller table".into(), ParserValue::Table(table_id));
    let original = source.table();
    original.raw_set("payload", -0.0).unwrap();
    original.raw_set("self", original.clone()).unwrap();
    source.lookup.raw_set("caller table", original).unwrap();
    let owner = ModifierParserCatalog::new(data).unwrap();
    let catalog = ParserProgramCatalog::new(plan.catalog().data().clone(), owner).unwrap();
    let plan = poe_optimizer_engine::parser_program::CompiledParserPrograms::new(&catalog).unwrap();
    source.pair(
        &plan,
        Target::Grant,
        MultiValue::from_vec(vec![
            source.text("caller table skill"),
            Value::Number(1.0),
            Value::Nil,
        ]),
        "definition-backed cyclic skillId graph",
    );
    source.pair(
        &plan,
        Target::Property,
        MultiValue::from_vec(vec![
            Value::Number(1.0),
            Value::Nil,
            source.text("quality"),
            source.text("caller table"),
            Value::Nil,
        ]),
        "truthy table lookup",
    );
}

#[test]
fn every_original_lookup_entry_is_exercised_by_all_four_raw_bodies() {
    let plan = source::plan();
    let source = Source::new();
    let entries = plan
        .catalog()
        .owner()
        .dictionary(ParserDictionary::GemIdLookup);
    let mut count = 0;
    for name in entries.fields.keys() {
        for target in Target::ALL {
            let args = match target {
                Target::Property => vec![
                    Value::Number(2.0),
                    Value::Nil,
                    source.text("quality"),
                    source.text(name),
                    source.text("strength"),
                ],
                Target::Grant => vec![source.text(name), Value::Number(3.0), Value::Boolean(false)],
                Target::Plain => vec![Value::Nil, source.text(name)],
                Target::Level => vec![Value::Number(4.0), Value::Nil, source.text(name)],
            };
            source.pair(&plan, target, MultiValue::from_vec(args), name);
            count += 1;
        }
    }
    assert_eq!(count, 3844);
    eprintln!("all original lookup records: {count} full raw output graph pairs");
}

#[test]
fn live_warm_controls_execute_original_bodies_and_constructor_without_cache_hits() {
    use mlua::{Function, Table};
    use poe_optimizer_engine::parser_program::ProgramLimits;
    let plan = source::plan();
    for (target, name) in [
        (Target::Property, "fireball"),
        (Target::Property, "callerword"),
        (Target::Property, "caller word"),
        (Target::Grant, "fireball"),
        (Target::Grant, "missing caller"),
        (Target::Plain, "fireball"),
        (Target::Level, "fireball"),
    ] {
        // A fresh authenticated VM gives each control an independent trace history.
        let source = Source::new();
        let lua = &source.factory.public.source.lua;
        let driver: Table = lua
            .load(include_str!("support/parser_program_source_warm.lua"))
            .set_name("@test-only-live-program-driver")
            .eval()
            .unwrap();
        let args = match target {
            Target::Property => vec![
                Value::Number(2.0),
                Value::Nil,
                source.text("quality"),
                source.text(name),
                source.text("intelligence"),
            ],
            Target::Grant => vec![source.text(name), Value::Number(3.0), Value::Boolean(false)],
            Target::Plain => vec![Value::Nil, source.text(name)],
            Target::Level => vec![Value::Number(4.0), Value::Nil, source.text(name)],
        };
        let native_input = source::from_source(
            &source
                .factory
                .public
                .capture(MultiValue::from_vec(args.clone()))
                .unwrap(),
        );
        let packed = lua.create_table().unwrap();
        for (i, value) in args.into_iter().enumerate() {
            packed.raw_set(i + 1, value).unwrap();
        }
        let required = lua.create_table().unwrap();
        required
            .raw_set("body", source.function(target).clone())
            .unwrap();
        if name != "missing caller" {
            required
                .raw_set("constructor", source.factory.create_mod.clone())
                .unwrap();
        }
        if matches!(target, Target::Plain | Target::Level) {
            required.raw_set("callee", source.grant.clone()).unwrap();
        }
        driver
            .get::<Function>("begin")
            .unwrap()
            .call::<()>(())
            .unwrap();
        let values: MultiValue = driver
            .get::<Function>("run")
            .unwrap()
            .call((source.function(target).clone(), packed))
            .unwrap();
        let evidence: Table = driver
            .get::<Function>("finish")
            .unwrap()
            .call(required.clone())
            .unwrap();
        let found: Table = evidence.get("found").unwrap();
        assert!(evidence.get::<usize>("live").unwrap() > 0);
        for row in required.pairs::<String, Function>() {
            let (key, _) = row.unwrap();
            assert!(
                found.get::<bool>(key.as_str()).unwrap_or(false),
                "{target:?}/{name}: original {key} absent from completed live trace"
            );
        }
        let expected = source.factory.public.capture(values).unwrap();
        let native = plan
            .execute(
                target.id(plan.catalog().owner()),
                &native_input,
                ProgramLimits::default(),
            )
            .unwrap();
        assert_eq!(
            source::native_graph(native.graph()),
            expected,
            "warm {target:?}/{name}"
        );
        eprintln!(
            "live original {target:?}/{name}: {} completed live traces, {} stops, {} aborts",
            evidence.get::<usize>("live").unwrap(),
            evidence.get::<usize>("stop").unwrap(),
            evidence.get::<usize>("abort").unwrap()
        );
    }
}

#[test]
fn original_duplicate_assignment_stores_are_observed_separately_from_target_parity() {
    let lua = mlua::Lua::new();
    lua.load("jit.off(); jit.flush()").exec().unwrap();
    let local: i64 = lua.load("local a=0; a,a=1,2; return a").eval().unwrap();
    let mixed: (i64, i64) = lua
        .load("local a,b=0,0; a,b,a=1,2,3; return a,b")
        .eval()
        .unwrap();
    let table: i64 = lua
        .load("local t={}; t.x,t.x=1,2; return t.x")
        .eval()
        .unwrap();
    assert_eq!(local, 1);
    assert_eq!(mixed, (1, 2));
    assert_eq!(table, 1);
    eprintln!(
        "original interpreted duplicate stores: a,a=1,2 -> {local}; a,b,a=1,2,3 -> {mixed:?}; t.x,t.x=1,2 -> {table}; source-only lowering requirement"
    );
}

#[test]
fn opaque_constructor_values_are_identifiers_not_function_graph_equivalence() {
    use poe_optimizer_engine::parser_program::*;
    let source = Source::new();
    let plan = source::injected(&source, &[("caller", ParserValue::Text("CallerId".into()))]);
    let owner = plan.catalog().owner();
    let descriptor = &owner.data().callbacks[Target::Grant.id(owner).0 as usize - 1];
    let constructor = descriptor
        .upvalues
        .iter()
        .find_map(|u| match (u.name.as_str(), &u.value) {
            ("mod", ParserValue::Callback(id)) => Some(*id),
            _ => None,
        })
        .unwrap();
    let original = source.factory.create_mod.clone();
    // This pre-existing isolated extraction descriptor is deliberately opaque:
    // full ModTools lexical captures are observed, never normalized away.
    assert!(
        owner.data().callbacks[constructor.0 as usize - 1]
            .upvalues
            .is_empty()
    );
    assert!(original.info().num_upvalues > 0);
    let original_result: MultiValue = source
        .grant
        .call((source.text("caller"), 1, original.clone()))
        .unwrap();
    let result = original_result.front().unwrap().as_table().unwrap();
    let modifier: mlua::Table = result.get(1).unwrap();
    let payload: mlua::Table = modifier.get("value").unwrap();
    assert_eq!(
        payload
            .get::<mlua::Function>("noSupports")
            .unwrap()
            .to_pointer(),
        original.to_pointer()
    );
    let input = ProgramValueGraph {
        values: vec![
            ProgramValue::Bytes(b"caller".to_vec()),
            ProgramValue::Number(1.0),
            ProgramValue::Callback(constructor),
        ],
        tables: vec![],
    };
    let native = plan
        .execute(Target::Grant.id(owner), &input, ProgramLimits::default())
        .unwrap();
    assert!(
        native
            .graph()
            .tables
            .iter()
            .flat_map(|t| t.entries.iter())
            .any(|(k, v)| *k == ProgramValue::Bytes(b"noSupports".to_vec())
                && *v == ProgramValue::Callback(constructor))
    );
    // Resolving a callable table method is a separate unsupported capability.
    let receiver = source.table();
    receiver.raw_set("gsub", original).unwrap();
    let result: MultiValue = source.grant.call((receiver, 1, false)).unwrap();
    assert!(result.is_empty());
    let input = ProgramValueGraph {
        values: vec![
            ProgramValue::Table(ProgramTableId(1)),
            ProgramValue::Number(1.0),
            ProgramValue::Boolean(false),
        ],
        tables: vec![ProgramTable {
            entries: vec![(
                ProgramValue::Bytes(b"gsub".to_vec()),
                ProgramValue::Callback(constructor),
            )],
        }],
    };
    let native = plan
        .execute(Target::Grant.id(owner), &input, ProgramLimits::default())
        .unwrap_err();
    assert_eq!(native.kind, ProgramRuntimeErrorKind::UnsupportedCapability);
}
