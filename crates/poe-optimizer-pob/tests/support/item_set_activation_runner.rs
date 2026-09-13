//! Complete source Load receipts with a strictly narrower native activation boundary.
use super::{
    context, exact, graph, headless, materialization as material, raw_choices, rune_names, source,
};
use mlua::{Table, Value as LuaValue};
use poe_optimizer_core::{build_identity::BuildLineage, build_view::ViewRequest};
use poe_optimizer_engine::source_program::{
    ProgramTableId as Id, ProgramValue as V, ProgramValueGraph as Graph,
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    item_loading::assembly::AssemblyErrorKind,
    item_sets::{ItemActivationProgress, ItemSetLimits, ItemSetPhase},
    selected_view::{ResolveLimits, resolve_view},
};
use poe_optimizer_native::{
    CompiledGameData,
    items::{ItemPreparationLimits, prepare_authored_items},
};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};
const TEST: &str = "all_five_original_activation_population_and_component_histories";
const CHILD: &str = "POE_ITEM_SET_ACTIVATION_CHILD";
const OUTPUT: &str = "POE_ITEM_SET_ACTIVATION_OUTPUT";
const SCOPE: &str = "native activation through completed PopulateSlots at the exact original SetActiveItemSet pre-Sync boundary; declared mixed set/current/prior/child alias graph, selected IDs/notes/activation flags, duplicate-preserving ID-label choices, rune selected names and actual node-selection writes. Source UI order/indices are retained separately; no native Lua traversal certificate. Rune effect values/record aliases, parent control fields, exact trade transform identity, SyncLoadouts, actor equipment participation and whole Load completion are not established by this comparison.";
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn save(path: &Path, value: &Json) {
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
fn capture(t: Table) -> Graph {
    graph::capture(&[LuaValue::Table(t)]).unwrap()
}
fn value_graph(g: &Graph, name: &str) -> Json {
    let mut projected = g.clone();
    let V::Table(root) = g.values[0] else {
        panic!("root")
    };
    projected.values = vec![super::field(g, root, name)];
    exact(&projected)
}
fn node_graph(nodes: impl Iterator<Item = (i64, f64)>) -> Json {
    let graph = Graph {
        values: vec![V::Table(Id(1))],
        tables: vec![poe_optimizer_engine::source_program::ProgramTable {
            entries: nodes
                .map(|(k, v)| (V::Number(k as f64), V::Number(v)))
                .collect(),
        }],
    };
    exact(&graph)
}
fn original_functions(source: &Json) {
    for (name, path, first, last) in [
        ("items_load", "Classes/ItemsTab.lua", 1193, 1320),
        ("activate_set", "Classes/ItemsTab.lua", 1628, 1670),
        ("populate_slots", "Classes/ItemsTab.lua", 1705, 1709),
        ("populate_slot", "Classes/ItemSlotControl.lua", 117, 148),
        ("validity", "Classes/ItemsTab.lua", 2603, 2687),
    ] {
        let f = source["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["name"] == name)
            .unwrap();
        assert_eq!(f["first_line"], first);
        assert_eq!(f["last_line"], last);
        assert!(
            f["source"]
                .as_str()
                .unwrap()
                .replace('\\', "/")
                .ends_with(path)
        );
    }
}
fn selected_cases(xml: &str, derived: bool) -> Vec<material::Case> {
    material::cases(xml, derived)
        .into_iter()
        .filter(|c| {
            matches!(
                c.label,
                "original"
                    | "legacy_only"
                    | "missing_and_duplicate_ids"
                    | "mixed_rows"
                    | "duplicate_numeric_set_winner"
                    | "repeated_items_requires_activation_continuation"
            )
        })
        .collect()
}
fn public_native(xml: &str, data: &Arc<CompiledGameData>, source_after: &Graph) -> Json {
    let build = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([178; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let view = resolve_view(
        &build,
        data.snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    let prepared =
        prepare_authored_items(&build, &view, data, ItemPreparationLimits::default()).unwrap();
    prepared.validate_binding(&build, &view, data).unwrap();
    let snapshot = prepared.item_sets().map(|s| s.snapshot().unwrap());
    let completed = prepared
        .item_sets()
        .is_some_and(|s| s.phase() == ItemSetPhase::AwaitingSyncLoadouts);
    let comparison = if completed {
        let g = snapshot.as_ref().unwrap();
        let equal = headless(g, false) == headless(source_after, true);
        let nodes = prepared
            .activation_startup_jewels()
            .expect("actual owned node write context");
        let node_equal = node_graph(nodes.iter().map(|(k, v)| (*k as i64, *v)))
            == value_graph(source_after, "nodeJewels");
        json!({"available":true,"headless_graph_equal":equal,"node_write_graph_equal":node_equal,"raw_slot_arrays_equal":raw_choices(g)==raw_choices(source_after)})
    } else {
        json!({"available":false,"reason":"independent native inventory/activation has not reached the matching boundary"})
    };
    json!({"scope":"public production prepare_authored_items over original XML and exact owned compiled dataset, with no source values or traversal passed in","report":prepared.report(),"native_snapshot":snapshot.as_ref().map(exact),"comparison":comparison,"whole_load_claim":false})
}
fn compare(
    repo: &Path,
    directory: &Path,
    data: &Arc<CompiledGameData>,
    case: &material::Case,
) -> Json {
    fs::create_dir_all(directory).unwrap();
    fs::write(directory.join("input.xml"), &case.xml).unwrap();
    let observed = source::observe(repo, &directory.join("observed"), case, true);
    let observation = &observed.observation;
    let receipt = material::receipt(observation);
    save(&directory.join("source.json"), &receipt);
    original_functions(&receipt);
    observation
        .outcome
        .as_ref()
        .expect("complete original Load must return for this successful corpus");
    let before = material::states(observation, "activate_set", "before");
    let after = material::states(observation, "activate_set", "after_populate");
    assert!(
        !before.is_empty() && !after.is_empty(),
        "matching activation/population witness unavailable; inspect source.json"
    );
    let source_entry = capture(before[0].raw_get("value").unwrap());
    let source_after = capture(after[0].raw_get("value").unwrap());
    let call: u64 = before[0].raw_get("call_ordinal").unwrap();
    assert_eq!(after[0].raw_get::<u64>("call_ordinal").unwrap(), call);
    let pop_return: u64 = after[0].raw_get("event_ordinal").unwrap();
    let events = receipt["events"].as_array().unwrap();
    let activation = events.iter().find(|e| e["ordinal"] == call).unwrap();
    assert_eq!(activation["name"], "activate_set");
    assert_eq!(activation["event"], "call");
    let load = material::states(observation, "items_load", "before")[0]
        .raw_get::<u64>("call_ordinal")
        .unwrap();
    assert_eq!(activation["parent_call_ordinal"], load);
    assert_eq!(activation["load_call_ordinal"], load);
    let returned = events.iter().find(|e| e["ordinal"] == pop_return).unwrap();
    assert_eq!(returned["name"], "populate_slots");
    assert_eq!(returned["event"], "return");
    assert_eq!(returned["activation_call_ordinal"], call);
    let pop_call = returned["call_ordinal"].as_u64().unwrap();
    let entered = events.iter().find(|e| e["ordinal"] == pop_call).unwrap();
    assert_eq!(entered["direct_activation_caller"], true);
    assert_eq!(entered["parent_call_ordinal"], call);
    assert!(
        !events.iter().any(|e| e["name"] == "sync_loadouts"
            && e["event"] == "call"
            && e["ordinal"].as_u64().unwrap() > call
            && e["ordinal"].as_u64().unwrap() < pop_return),
        "matching snapshot must precede reached Sync"
    );
    let actual_order = events
        .iter()
        .filter(|e| {
            e["event"] == "call"
                && e["name"] == "populate_slot"
                && e["parent_call_ordinal"] == pop_call
        })
        .cloned()
        .collect::<Vec<_>>();
    assert!(!actual_order.is_empty());
    assert!(
        actual_order
            .iter()
            .all(|e| e["direct_populate_slots_caller"] == true)
    );
    let input_rows: Table = observation
        .report
        .raw_get("activation_input_contexts")
        .unwrap();
    let inputs = input_rows
        .sequence_values::<Table>()
        .map(Result::unwrap)
        .find(|r| r.raw_get::<u64>("call_ordinal").unwrap() == call)
        .unwrap();
    assert_eq!(
        inputs.raw_get::<u64>("receiver_token").unwrap(),
        before[0].raw_get::<u64>("receiver_token").unwrap()
    );
    let input: Table = inputs.raw_get("value").unwrap();
    save(
        &directory.join("source-input.json"),
        &json!({"call_ordinal":call,"graph":exact(&capture(input.clone())),"scope":"actual original activation-entry read set, not completed output; finite metadata ingress does not recover shared input row identities"}),
    );
    // This source-fed component probes the full tree/item/slot matrix. Its
    // explicit work allowance is separate from production preparation defaults.
    let component_limits = ItemSetLimits {
        max_steps: 50_000_000,
        ..ItemSetLimits::default()
    };
    let mut native = material::native_with_limits(
        &data.snapshot().item_assembly().policy().inventory,
        &case.xml,
        component_limits,
    );
    native.result.as_ref().unwrap();
    let constructor_source = material::states(observation, "items_load", "before")[0]
        .raw_get("value")
        .unwrap();
    let constructor_equal = material::common(capture(constructor_source), true, true, false)
        == material::common(native.constructor.clone(), false, true, false);
    let material_equal = material::common(source_entry, true, false, false)
        == material::common(native.state.snapshot().unwrap(), false, false, false);
    let actual_call: Table = observation
        .report
        .raw_get::<Table>("events")
        .unwrap()
        .raw_get(call)
        .unwrap();
    let requested: Table = actual_call.raw_get("requested_set").unwrap();
    assert_eq!(requested.raw_get::<String>("kind").unwrap(), "number");
    let requested: f64 = requested.raw_get("value").unwrap();
    assert_eq!(
        requested.to_bits(),
        native
            .state
            .pending_activation()
            .unwrap()
            .value()
            .unwrap()
            .to_bits()
    );
    // Each history starts from policy-derived constructor state. Source-projected
    // item/context inputs are component dependencies only, never a state snapshot.
    let root = context::activation_metadata(&input).unwrap();
    let mut context =
        context::Context::new(root, data.snapshot(), observed.parser.clone()).unwrap();
    observed.verify_parser();
    let progress = native.state.continue_activation(&mut context);
    observed.verify_parser();
    let graph = native.state.snapshot().unwrap();
    let complete = matches!(&progress, Ok(ItemActivationProgress::AwaitingSyncLoadouts));
    let comparison = if complete {
        let source = headless(&source_after, true);
        let target = headless(&graph, false);
        json!({"available":true,"headless_graph_equal":source==target,"source_headless":source,"native_headless":target,"node_write_graph_equal":node_graph(context.node_jewels.iter().map(|(k,v)|(*k,*v)))==value_graph(&source_after,"nodeJewels"),"raw_slot_arrays_equal":raw_choices(&source_after)==raw_choices(&graph)})
    } else {
        json!({"available":false,"graph_parity_claim":false,"reason":"native reached an explicit dependency or error before completed population"})
    };
    let progress_json = match &progress {
        Ok(p) => serde_json::to_value(p).unwrap(),
        Err(e) => json!({"error_kind":format!("{:?}",e.kind),"message":e.message}),
    };
    let repeat = if case.loads > 1 {
        let old = exact(&graph);
        let error = native
            .state
            .begin_load()
            .expect_err("later Load must not bypass SyncLoadouts");
        let same = old == exact(&native.state.snapshot().unwrap());
        assert_eq!(error.kind, AssemblyErrorKind::Unsupported);
        assert!(same);
        json!({"source_completed_load_count":case.loads,"native_begin_next_load_refused":true,"state_unchanged":same,"native_previous_activation_not_assumed_complete":true})
    } else {
        Json::Null
    };
    let final_graph = capture(observation.report.raw_get("finite_post_import").unwrap());
    let control = if !case.structural {
        let control = source::observe(repo, &directory.join("control"), case, false);
        control.observation.outcome.as_ref().unwrap();
        let control_receipt = material::receipt(&control.observation);
        save(&directory.join("control.json"), &control_receipt);
        let control_graph = capture(
            control
                .observation
                .report
                .raw_get("finite_post_import")
                .unwrap(),
        );
        json!({"exact_finite_graph_equal":exact(&final_graph)==exact(&control_graph),"declared_headless_key_equal":headless(&final_graph,true)==headless(&control_graph,true),"rune_selected_name_and_occurrences_equal":rune_names(&final_graph)==rune_names(&control_graph),"scope":"unhooked fresh original full-Load control compared only over declared headless fields plus rune selected name and duplicate-preserving name counts; exact raw graph difference retained separately, effect data/omitted fields/aliases outside key are not noninterference claims"})
    } else {
        json!({"available":false,"reason":"derived component case has no additional fresh unhooked control; original-five control scope is separate"})
    };
    let public = if !case.structural {
        public_native(&case.xml, data, &source_after)
    } else {
        Json::Null
    };
    let summary = json!({"label":case.label,"input_sha256":hash(case.xml.as_bytes()),"structurally_derived":case.structural,"scope":SCOPE,"source_load_returns":events.iter().filter(|e|e["event"]=="return"&&e["name"]=="items_load").count(),"source_activation_count":before.len(),"source_after_population_count":after.len(),
  "boundary":{"activation_call":call,"population_call":pop_call,"population_return":pop_return,"direct_original_parent_verified":true,"before_sync":true,"actual_parameter_projection_not_arity":true},
  "actual_source_population_order":actual_order,"source_constructor_equal":constructor_equal,"source_materialization_equal":material_equal,
  "component":{"scope":"source-fed pre-call inventory/tree/colors context; independently native-produced set state, native validity/rune/population; original parser dependency separately labelled; explicit component-only work allowance for the full tree/item/slot matrix","item_set_limits":component_limits,"limits_scope":"component fixture only; other ItemSetLimits fields and the separate public production defaults are unchanged","source_order_passed_to_native":false,"native_inventory_claim":false,"native_whole_load_claim":false,"progress":progress_json,"comparison":comparison,"native_graph":exact(&graph),"native_raw_slot_arrays":raw_choices_if_complete(&graph,complete),"native_usage":native.state.usage(),"rune_preparation":context.rune_preparation,"original_parser_dependency_calls":context.parser_calls,"native_probe_calls":context.validity_calls,"node_jewels":context.node_jewels},
  "source_raw_slot_arrays":raw_choices(&source_after),"public_native_original":public,"repeat":repeat,"control":control});
    save(&directory.join("comparison.json"), &summary);
    assert!(
        constructor_equal && material_equal,
        "constructor/materialization mismatch: {}",
        directory.display()
    );
    if let Err(e) = progress {
        assert_eq!(
            e.kind,
            AssemblyErrorKind::Unsupported,
            "native error inconsistent with successful full original Load: {e}"
        );
    }
    if complete {
        assert_eq!(
            summary["component"]["comparison"]["headless_graph_equal"],
            true,
            "activation graph mismatch: {}",
            directory.display()
        );
        assert_eq!(
            summary["component"]["comparison"]["node_write_graph_equal"],
            true,
            "node write mismatch: {}",
            directory.display()
        );
    }
    if !case.structural {
        assert_eq!(
            summary["control"]["declared_headless_key_equal"],
            true,
            "control projection mismatch: {}",
            directory.display()
        );
        assert_eq!(
            summary["control"]["rune_selected_name_and_occurrences_equal"],
            true
        );
        if summary["public_native_original"]["comparison"]["available"] == true {
            assert_eq!(
                summary["public_native_original"]["comparison"]["headless_graph_equal"],
                true
            );
            assert_eq!(
                summary["public_native_original"]["comparison"]["node_write_graph_equal"],
                true
            );
        }
    }
    summary
}
fn raw_choices_if_complete(g: &Graph, complete: bool) -> Json {
    if complete { raw_choices(g) } else { Json::Null }
}
pub fn run() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let input = repo.join("tests/fixtures/builds/breadth-20260908");
    let index: Json = serde_json::from_slice(&fs::read(input.join("index.json")).unwrap()).unwrap();
    let output = std::env::var_os(OUTPUT)
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("runs/r2ah-equipment-activation-01/source"));
    fs::create_dir_all(&output).unwrap();
    if let Ok(name) = std::env::var(CHILD) {
        let entry = index["builds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["xml"] == name)
            .unwrap();
        let xml = fs::read_to_string(input.join(&name)).unwrap();
        assert_eq!(hash(xml.as_bytes()), entry["xml_sha256"]);
        let data = CompiledGameData::bundled().unwrap();
        let directory = output.join(&name);
        fs::create_dir_all(&directory).unwrap();
        let mut cases = Vec::new();
        for case in selected_cases(&xml, name == index["builds"][0]["xml"].as_str().unwrap()) {
            cases.push(compare(&repo, &directory.join(case.label), &data, &case));
        }
        let report = json!({"xml":name,"xml_sha256":entry["xml_sha256"],"package_sha256":poe_optimizer_data::game_data::bundled_package_sha256(),"source_files":source_hashes(&repo),"observer_sha256":hash(source::OBSERVER.as_bytes()),"cases":cases,"scope":SCOPE});
        save(&output.join(format!("{name}.json")), &report);
        return;
    }
    let mut children = Vec::new();
    let mut complete = 0;
    let mut cases = 0;
    for entry in index["builds"].as_array().unwrap() {
        let name = entry["xml"].as_str().unwrap();
        let mut process = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--nocapture"])
            .env(CHILD, name)
            .env(OUTPUT, &output)
            .current_dir(repo.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(
                fs::File::create(output.join(format!("{name}.stdout.log"))).unwrap(),
            ))
            .stderr(Stdio::from(
                fs::File::create(output.join(format!("{name}.stderr.log"))).unwrap(),
            ))
            .spawn()
            .unwrap();
        let start = Instant::now();
        let status = loop {
            if let Some(status) = process.try_wait().unwrap() {
                break status;
            }
            if start.elapsed() > Duration::from_secs(900) {
                process.kill().unwrap();
                let _ = process.wait();
                panic!("activation source child timeout {name}");
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        assert!(
            status.success(),
            "activation source child failed {name}; inspect {}",
            output.display()
        );
        let report: Json =
            serde_json::from_slice(&fs::read(output.join(format!("{name}.json"))).unwrap())
                .unwrap();
        let rows = report["cases"].as_array().unwrap();
        cases += rows.len();
        complete += rows
            .iter()
            .filter(|r| r["component"]["comparison"]["available"] == true)
            .count();
        children.push(json!({"xml":name,"exit":status.code()}));
    }
    save(
        &output.join("summary.json"),
        &json!({"children":children,"cases":cases,"component_population_completions":complete,"originals":5,"scope":SCOPE,"unavailable_is_not_graph_parity":true,"source_methods_replaced":false,"source_traversal_imported":false}),
    );
    assert!(
        complete > 0,
        "no available source/native population comparison; cannot certify a fixture made entirely of frontiers"
    );
}
fn source_hashes(repo: &Path) -> BTreeMap<&'static str, String> {
    [
        "Classes/ItemsTab.lua",
        "Classes/ItemSlotControl.lua",
        "Classes/DropDownControl.lua",
        "Modules/Build.lua",
    ]
    .into_iter()
    .map(|p| {
        (
            p,
            hash(&fs::read(repo.join("vendor/path-of-building-poe2/src").join(p)).unwrap()),
        )
    })
    .collect()
}
