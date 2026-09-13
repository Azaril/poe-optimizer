//! Actual five-original native assembly comparison. Source parsing is an explicit
//! isolation dependency in one lane; a second lane uses the built-in native parser.
#[path = "item_assembly_native.rs"]
mod dependencies;
#[path = "item_assembly_histories.rs"]
mod histories;
#[allow(dead_code)]
#[path = "item_assembly_graph.rs"]
mod observation;
#[allow(dead_code)]
#[path = "configuration_preparation_source.rs"]
mod source;
use mlua::{Function, Lua, Table, Value};
use poe_optimizer_data::game_data::bundled_snapshot;
use poe_optimizer_engine::source_program::{ProgramValue, ProgramValueGraph};
use poe_optimizer_import::{
    item_loading::*,
    item_source::{ItemSourceKind, ItemSourceNode},
    source_xml::PobContentEntry,
};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    rc::Rc,
    time::{Duration, Instant},
};
const TEST: &str = "all_five_actual_accessories_match_owned_native_assembly";
const CHILD: &str = "POE_ITEM_ASSEMBLY_CHILD";
const OUTPUT: &str = "POE_ITEM_ASSEMBLY_OUTPUT";
const OBSERVER: &str = include_str!("item_assembly_source.lua");
// Explicit consumer contract agreed before comparisons; not selected from native output.
pub(super) const FIELDS: &[&str] = &[
    "name",
    "type",
    "id",
    "baseModList",
    "modList",
    "slotModList",
    "buffModList",
    "rangeLineList",
    "grantedSkills",
    "canSocketJewelBase",
    "requirements",
    "sockets",
    "buffModLines",
    "enchantModLines",
    "runeModLines",
    "classRequirementModLines",
    "implicitModLines",
    "explicitModLines",
    "modSource",
    "hasModTags",
    "classRestriction",
    "quality",
    "craftedQuality",
    "spiritValue",
    "charmLimit",
    "socketedAugmentTypeOverride",
    "socketedSoulCoreTypes",
    "socketedIdolsUseBondedModifiers",
    "socketedSoulCoreEffectModifier",
    "socketedRuneEffectModifier",
    "socketedAugmentItemEffectModifier",
    "socketedJewelEffectModifier",
    "activeBondedState",
];
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
const ROW_GROUPS: &[&str] = &[
    "buffModLines",
    "enchantModLines",
    "runeModLines",
    "classRequirementModLines",
    "implicitModLines",
    "explicitModLines",
    "rangeLineList",
];
// Fixed before execution: LoadedModLine consumer fields and original LineFlags.
const ROW_FIELDS: &[&str] = &[
    "line",
    "modList",
    "bondedModList",
    "modTags",
    "range",
    "valueScalar",
    "corruptedRange",
    "extra",
    "disabled",
    "bonded",
    "augmentType",
    "order",
    "runeCount",
    "displayValueScalar",
    "socketedRuneEffectAlreadyApplied",
    "socketedAugmentTypeOverride",
    "socketedSoulCoreType",
    "variantList",
    "versionList",
    "variantGroupList",
    "crafted",
    "custom",
    "enchant",
    "fractured",
    "implicit",
    "desecrated",
    "mutated",
    "rune",
    "unscalable",
    "prefix",
    "suffix",
];
fn project(mut graph: ProgramValueGraph) -> (ProgramValueGraph, Vec<Json>) {
    let ProgramValue::Table(root) = graph.values[0] else {
        panic!("assembled Item root is not a table")
    };
    let mut rows = BTreeSet::new();
    for (key, value) in &graph.tables[root.0 as usize - 1].entries {
        if matches!(key, ProgramValue::Bytes(key) if ROW_GROUPS.iter().any(|field| key == field.as_bytes()))
            && let ProgramValue::Table(group) = value
        {
            for (index, row) in &graph.tables[group.0 as usize - 1].entries {
                if matches!(index, ProgramValue::Number(_))
                    && let ProgramValue::Table(row) = row
                {
                    rows.insert(*row);
                }
            }
        }
    }
    let mut omitted = Vec::new();
    for row in rows {
        graph.tables[row.0 as usize - 1].entries.retain(|(key,value)| {
            let retain = matches!(key, ProgramValue::Bytes(key) if ROW_FIELDS.iter().any(|field| key == field.as_bytes()));
            if !retain { omitted.push(json!({"transport_table":row.0,"key":format!("{key:?}"),"value_kind":match value { ProgramValue::Nil=>"nil",ProgramValue::Boolean(_)=>"boolean",ProgramValue::Number(_)=>"number",ProgramValue::Bytes(_)=>"bytes",ProgramValue::Table(_)=>"table",_=>"unrepresented" }})); }
            retain
        });
    }
    graph.tables[root.0 as usize - 1].entries.retain(|(key,_)| matches!(key, ProgramValue::Bytes(key) if FIELDS.iter().any(|field| key == field.as_bytes())));
    (graph, omitted)
}
pub(super) fn compare_graph(
    assembled: &assembly::AssembledItem,
    event: &Table,
    label: &str,
) -> Json {
    let after: Table = event.raw_get("after").unwrap();
    assert!(
        after.raw_get::<bool>("available").unwrap(),
        "{label}: source representation unavailable: {:?}",
        after.raw_get::<Option<String>>("reason").unwrap()
    );
    let omitted_roots: Table = after.raw_get("omitted_root_fields").unwrap();
    let omitted_roots = omitted_roots.sequence_values::<Table>().map(|row| { let row=row.unwrap(); json!({"key":row.raw_get::<String>("key").unwrap(),"value_kind":row.raw_get::<String>("value_kind").unwrap()}) }).collect::<Vec<_>>();
    let classes: Table = after.raw_get("class_projections").unwrap();
    assert!(
        classes.raw_len() <= observation::MAX_TABLES,
        "comparison class receipt bound"
    );
    let class_projections = classes.sequence_values::<Table>().map(|row| { let row=row.unwrap();
        let keys: Table=row.raw_get("omitted_infrastructure").unwrap();
        json!({"class":row.raw_get::<String>("class").unwrap(),"omitted_infrastructure":keys.sequence_values::<Table>().map(|key| { let key=key.unwrap(); json!({"key":key.raw_get::<String>("key").unwrap(),"value_kind":key.raw_get::<String>("value_kind").unwrap(),"identity_verified":key.raw_get::<bool>("identity_verified").unwrap()}) }).collect::<Vec<_>>()}) }).collect::<Vec<_>>();
    let (expected, omitted_source) = project(
        observation::capture(&[Value::Table(after.raw_get("root").unwrap())])
            .expect("source comparison graph must be represented within explicit bounds"),
    );
    let (actual, omitted_native) = project(assembled.snapshot().unwrap());
    let actual = observation::canonical(&actual).expect("native comparison canonical bound");
    let expected = observation::canonical(&expected).expect("source comparison canonical bound");
    if actual != expected {
        let output = std::env::var_os(OUTPUT)
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../runs/r2y-item-assembly-01/source-parity")
            });
        let directory = output.join("mismatches");
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join(format!("{}.json", hash(label.as_bytes())));
        fs::write(&path,serde_json::to_vec_pretty(&json!({"label":label,"actual":actual,"expected":expected,"omitted_source_row_fields":omitted_source,"omitted_native_row_fields":omitted_native,"omitted_source_root_fields":omitted_roots,"source_class_projections":class_projections})).unwrap()).unwrap();
        panic!(
            "{label}: declared assembly-field/row-field graph contract differs; {}",
            path.display()
        );
    }
    json!({"output_sha256":hash(actual.to_string().as_bytes()),"usage":format!("{:?}",assembled.usage()),"omitted_source_row_fields":omitted_source,"omitted_native_row_fields":omitted_native,"omitted_source_root_fields":omitted_roots,"source_class_projections":class_projections})
}
fn replay<'a>(
    node: &ItemSourceNode<'_>,
    snapshot: &'a poe_optimizer_data::game_data::GameDataSnapshot,
    provider: &mut impl ItemLoadProvider,
) -> (ItemLoadMachine<'a>, Vec<Json>) {
    let mut errors = Vec::new();
    let mut machine = ItemLoadMachine::new(snapshot.item_loading());
    machine.set_xml_attributes(
        &node
            .element()
            .attributes()
            .iter()
            .map(|a| (a.name().to_owned(), a.value().decoded().to_owned()))
            .collect(),
    );
    for entry in node.ordered_content().consumed() {
        if matches!(
            machine.status(),
            ItemLoadStatus::Pending | ItemLoadStatus::SourceError
        ) {
            break;
        }
        match entry {
            PobContentEntry::Text { text, .. } => {
                if let Err(e) = machine.apply_text(text, provider) {
                    errors.push(replay_error(&machine, "ParseRaw", &e));
                    break;
                }
            }
            PobContentEntry::Element { child_index } => {
                let child = &node.children()[*child_index];
                if child.kind() == ItemSourceKind::ModRange
                    && let Err(e) = machine.apply_mod_range(
                        child.element().attribute("id").map(|a| a.decoded()),
                        child.element().attribute("range").map(|a| a.decoded()),
                    )
                {
                    errors.push(replay_error(&machine, "ModRange", &e));
                    break;
                }
            }
        }
    }
    if errors.is_empty()
        && let Err(e) = machine.finish_load(provider)
    {
        errors.push(replay_error(&machine, "final assembly", &e));
    }
    if !errors.is_empty() {
        assert!(
            machine.assembled().is_none(),
            "an error must not authorize registration"
        );
    }
    (machine, errors)
}
fn replay_error(machine: &ItemLoadMachine<'_>, stage: &str, error: &ItemLoadError) -> Json {
    let classification = if machine.status() == ItemLoadStatus::SourceError {
        "source_error"
    } else if machine.status() == ItemLoadStatus::Pending
        && machine.pending().is_some_and(|pending| {
            pending.kind == DependencyKind::Assembly
                && error.to_string() == format!("native item loading: {}", pending.message)
        })
    {
        "assembly_resource_error_with_pending_status"
    } else {
        "returned_error_without_public_typed_classification"
    };
    json!({"stage":stage,"classification":classification,"message":error.to_string(),"status":format!("{:?}",machine.status()),"pending":machine.pending()})
}
fn eligible(item: &Table) -> bool {
    let base: Table = item.raw_get("base").unwrap();
    for field in ["weapon", "armour", "flask", "charm"] {
        if !matches!(
            base.raw_get::<Value>(field).unwrap(),
            Value::Nil | Value::Boolean(false)
        ) {
            return false;
        }
    }
    item.raw_get::<String>("type").unwrap() != "Jewel"
}
fn compare(machine: &ItemLoadMachine<'_>, event: &Table, label: &str) -> Json {
    if let Some(assembled) = machine.assembled() {
        assert!(assembled.is_complete());
        assert_eq!(machine.status(), ItemLoadStatus::Complete);
        assert!(event.raw_get::<bool>("completed").unwrap(), "{label}");
        let graph = compare_graph(assembled, event, label);
        let retained = assembled.clone();
        assert!(retained.shares_storage_with(assembled));
        json!({"status":"complete","graph":graph})
    } else {
        assert_ne!(
            machine.status(),
            ItemLoadStatus::SourceError,
            "{label}: original succeeded but native raised a source error"
        );
        assert_ne!(
            machine.status(),
            ItemLoadStatus::Complete,
            "{label}: complete without owned final assembly"
        );
        json!({"status":format!("{:?}",machine.status()),"pending":machine.pending(),"has_owned_prefix":machine.assembly_progress().is_some()})
    }
}
// A separate complete Lua host, same pinned source/XML and JIT import mode.
// It installs no Call hook and runs no native replay. The comparison is a finite
// post-import witness, not universal noninterference or cross-host pointer identity.
fn post_import(lua: &Lua, capture: &Table) -> Json {
    let build: Table = lua.globals().raw_get("build").unwrap();
    let tab: Table = build.raw_get("itemsTab").unwrap();
    let items: Table = tab.raw_get("items").unwrap();
    let snapshot: Function = capture.raw_get("snapshot").unwrap();
    let mut graphs = BTreeMap::new();
    let mut compared = 0usize;
    let mut unavailable = 0usize;
    for (index, row) in items.pairs::<Value, Table>().enumerate() {
        assert!(index < 4096, "post-import Item count bound");
        let (id, item) = row.unwrap();
        let id = match id {
            Value::Integer(n) => n.to_string(),
            Value::Number(n) if n.is_finite() && n.fract() == 0.0 => format!("{n:.0}"),
            _ => panic!("original Item inventory key"),
        };
        let observed: Table = snapshot.call(item).unwrap();
        let value = if observed.raw_get::<bool>("available").unwrap() {
            let (graph, _) = project(
                observation::capture(&[Value::Table(observed.raw_get("root").unwrap())]).unwrap(),
            );
            let canonical = observation::canonical(&graph).unwrap();
            compared += 1;
            json!({"available":true,"graph_sha256":hash(canonical.to_string().as_bytes())})
        } else {
            unavailable += 1;
            json!({"available":false,"reason":observed.raw_get::<String>("reason").unwrap()})
        };
        assert!(graphs.insert(id, value).is_none());
    }
    let order: Table = tab.raw_get("itemOrderList").unwrap();
    let order =
        observation::canonical(&observation::capture(&[Value::Table(order)]).unwrap()).unwrap();
    let main: Table = build
        .raw_get::<Table>("calcsTab")
        .unwrap()
        .raw_get("mainOutput")
        .unwrap();
    let mut scalars = BTreeMap::new();
    let mut omitted = BTreeMap::<String, usize>::new();
    for (index, row) in main.pairs::<String, Value>().enumerate() {
        assert!(index < 32768, "main output field count bound");
        let (name, value) = row.unwrap();
        if matches!(
            value,
            Value::Nil
                | Value::Boolean(_)
                | Value::Integer(_)
                | Value::Number(_)
                | Value::String(_)
        ) {
            let canonical =
                observation::canonical(&observation::capture(&[value]).unwrap()).unwrap();
            scalars.insert(name, canonical);
        } else {
            *omitted.entry(value.type_name().to_owned()).or_default() += 1;
        }
    }
    json!({"items":graphs,"item_order":order,"main_output_scalars":scalars,"omitted_main_output_value_kinds":omitted,
        "item_graphs_compared":compared,"item_graphs_unavailable":unavailable,
        "scope":{"declared_assembly_field_row_field_graph_contract":true,"joint_identity_across_item_roots":false,"unknown_item_graphs_count_as_parity":false,"whole_build_output_graph":false}})
}
fn control(repo: &Path, directory: &Path, xml: &str) -> Json {
    let module = Rc::new(RefCell::new(None::<Table>));
    let capture = Rc::new(RefCell::new(None::<Table>));
    let before_source = |lua: &Lua| {
        *module.borrow_mut() = Some(
            lua.load(OBSERVER)
                .set_name("@item_assembly_source.lua")
                .eval()?,
        );
        Ok(())
    };
    let before_build = |lua: &Lua| {
        let observed: Table = module
            .borrow()
            .as_ref()
            .unwrap()
            .raw_get::<Function>("start")?
            .call((lua.create_sequence_from(FIELDS.iter().copied())?, false))?;
        let finish: Function = observed.raw_get("finish")?;
        *capture.borrow_mut() = Some(observed);
        Ok(finish)
    };
    let after = |lua: &Lua| Ok(post_import(lua, capture.borrow().as_ref().unwrap()));
    let host = directory.join("control-host");
    fs::create_dir_all(&host).unwrap();
    let result = source::observe_with_build_hook_unwrapped(
        &repo.join("vendor/path-of-building-poe2"),
        &host,
        xml,
        None,
        false,
        Some(&before_source),
        Some(&before_build),
        Some(&after),
    )
    .unwrap();
    fs::write(
        directory.join("control-report.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    result
}
fn child(repo: &Path, output: &Path, entry: &Json) {
    let fixture = repo
        .join("tests/fixtures/builds/breadth-20260908")
        .join(entry["xml"].as_str().unwrap());
    let bytes = fs::read(&fixture).unwrap();
    assert_eq!(hash(&bytes), entry["xml_sha256"].as_str().unwrap());
    let xml = std::str::from_utf8(&bytes).unwrap();
    let directory = output.join(entry["xml"].as_str().unwrap().trim_end_matches(".xml"));
    fs::create_dir_all(&directory).unwrap();
    fs::create_dir_all(directory.join("host")).unwrap();
    let control = control(repo, &directory, xml);
    let module = Rc::new(RefCell::new(None::<Table>));
    let capture = Rc::new(RefCell::new(None::<Table>));
    let original_parser = Rc::new(RefCell::new(None::<(Table, Function)>));
    let before_source = |lua: &Lua| {
        *module.borrow_mut() = Some(
            lua.load(OBSERVER)
                .set_name("@item_assembly_source.lua")
                .eval()?,
        );
        Ok(())
    };
    let before_build = |lua: &Lua| {
        let library: Table = lua.globals().raw_get("modLib")?;
        *original_parser.borrow_mut() = Some((library.clone(), library.raw_get("parseMod")?));
        let classes: Table = lua
            .globals()
            .raw_get::<Table>("common")?
            .raw_get("classes")?;
        for (class, name, first, last) in [
            ("Item", "ParseRaw", 468, 1803),
            ("Item", "BuildModList", 2694, 2863),
            ("ItemsTab", "Load", 1193, 1320),
        ] {
            let f: Function = classes.raw_get::<Table>(class)?.raw_get(name)?;
            let info = f.info();
            assert_eq!(info.line_defined, Some(first));
            assert_eq!(info.last_line_defined, Some(last));
            assert!(
                info.source
                    .as_deref()
                    .unwrap()
                    .replace('\\', "/")
                    .ends_with(&format!("Classes/{class}.lua"))
            );
        }
        let observed: Table = module
            .borrow()
            .as_ref()
            .unwrap()
            .raw_get::<Function>("start")?
            .call(lua.create_sequence_from(FIELDS.iter().copied())?)?;
        let finish = observed.raw_get::<Function>("finish")?;
        *capture.borrow_mut() = Some(observed);
        Ok(finish)
    };
    let after = |lua: &Lua| {
        let capture = capture.borrow();
        let capture = capture.as_ref().unwrap();
        let observed: Table = capture.raw_get::<Function>("report")?.call(())?;
        let post_import = post_import(lua, capture);
        let control_equal = post_import == control["additional_observation"];
        fs::write(directory.join("observer-control.json"),serde_json::to_vec_pretty(&json!({"equal":control_equal,"control":control["additional_observation"],"observed":post_import,
            "scope":"two fresh complete Lua hosts in one process; equal JIT-off import mode; hook is only enabled in observed host"})).unwrap())?;
        assert!(
            control_equal,
            "observed/control post-import witness differs; inspect observer-control.json"
        );
        let events: Table = observed.raw_get("events")?;
        let mut finals = BTreeMap::new();
        for event in events.sequence_values::<Table>() {
            let event = event?;
            if event.raw_get::<String>("phase")? != "final_load" {
                continue;
            }
            let node: Table = capture
                .raw_get::<Function>("node")?
                .call(event.raw_get::<u32>("parent_node_token")?)?;
            let id: String = node.raw_get::<Table>("attrib")?.raw_get("id")?;
            assert!(
                finals.insert(id, event).is_none(),
                "original corpus IDs are unique; do not collapse duplicate histories"
            );
        }
        let projected = poe_optimizer_import::item_source::project_xml(xml).unwrap();
        let snapshot = bundled_snapshot().unwrap();
        let (parser_library, parser) = original_parser.borrow().as_ref().unwrap().clone();
        assert_eq!(lua.globals().raw_get::<Table>("modLib")?, parser_library);
        assert_eq!(parser_library.raw_get::<Function>("parseMod")?, parser);
        let mut seen = BTreeSet::new();
        let mut rows = Vec::new();
        let mut source_complete = 0;
        let mut builtin_complete = 0;
        for container in projected.containers() {
            for node in container
                .children()
                .iter()
                .filter(|n| n.kind() == ItemSourceKind::Item)
            {
                let id = node.element().attribute("id").unwrap().decoded();
                assert!(seen.insert(id.to_owned()));
                let event = finals
                    .get(id)
                    .expect("actual original final assembly event missing");
                let item: Table = capture
                    .raw_get::<Function>("item")?
                    .call(event.raw_get::<u32>("item_token")?)?;
                let source_eligible = eligible(&item);
                if !source_eligible {
                    rows.push(json!({"id":id,"source_range":node.element().source_range(),"scope":"local_item_family_dependency"}));
                    continue;
                }
                let dependencies = dependencies::OriginalParser {
                    function: parser.clone(),
                    calls: 0,
                };
                let mut isolated =
                    NativeItemLoadProvider::with_native_assembly(&snapshot, dependencies);
                let (machine, isolated_errors) = replay(node, &snapshot, &mut isolated);
                let isolated_result = compare(
                    &machine,
                    event,
                    &format!(
                        "{} Item{id} source-parser/native-assembly",
                        fixture.display()
                    ),
                );
                if machine.assembled().is_some() {
                    source_complete += 1;
                }
                let mut builtin = BuiltinItemLoadProvider::new(&snapshot);
                let (machine, builtin_errors) = replay(node, &snapshot, &mut builtin);
                let builtin_result = compare(
                    &machine,
                    event,
                    &format!("{} Item{id} builtin", fixture.display()),
                );
                if machine.assembled().is_some() {
                    builtin_complete += 1;
                }
                rows.push(json!({"id":id,"source_range":node.element().source_range(),"scope":"eligible_accessory","original_parser_native_assembly":isolated_result,"builtin_native_pipeline":builtin_result,"source_dependency_calls":isolated.dependencies().calls,"source_parser_lane_errors":isolated_errors,"builtin_lane_errors":builtin_errors}));
            }
        }
        assert_eq!(
            seen.len(),
            finals.len(),
            "all original Item occurrences remain in denominator"
        );
        assert!(
            source_complete > 0,
            "no native accessory assembly completed on this original"
        );
        assert!(
            builtin_complete > 0,
            "no built-in native accessory completed on this original"
        );
        let history_report =
            histories::run(lua, module.borrow().as_ref().unwrap(), &parser, &snapshot);
        assert_eq!(parser_library.raw_get::<Function>("parseMod")?, parser);
        Ok(
            json!({"observer_control_equal":control_equal,"observer_control_scope":"declared finite post-import witness; separate fresh Lua hosts; no hook in control","post_import":post_import,"directed_histories":history_report,"items":rows,"item_count":seen.len(),"source_parser_lane_complete":source_complete,"builtin_lane_complete":builtin_complete,
            "scope":{"complete_native_build":false,"finite_accessory_only":true,"source_assembly_results_used_as_dependency":false,"arbitrary_input_alias_recovery":false,"registered_inventory_execution":false,"actual_dependency_arity_observed":false,"dependency_order_parity":false,"comparison":"declared assembly-field/row-field graph contract","root_fields":FIELDS,"row_fields":ROW_FIELDS}}),
        )
    };
    let report = source::observe_with_build_hook_unwrapped(
        &repo.join("vendor/path-of-building-poe2"),
        &directory.join("host"),
        xml,
        None,
        false,
        Some(&before_source),
        Some(&before_build),
        Some(&after),
    )
    .unwrap();
    assert_eq!(
        report["selected"], control["selected"],
        "original selected views changed under hook"
    );
    assert_eq!(report["source_hash"], control["source_hash"]);
    fs::write(
        directory.join("report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    assert_eq!(fs::read(fixture).unwrap(), bytes);
}
pub fn run() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let index: Json = serde_json::from_slice(
        &fs::read(repo.join("tests/fixtures/builds/breadth-20260908/index.json")).unwrap(),
    )
    .unwrap();
    let output = std::env::var_os(OUTPUT)
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("runs/r2y-item-assembly-01/source-parity"));
    fs::create_dir_all(&output).unwrap();
    if let Ok(name) = std::env::var(CHILD) {
        let entry = index["builds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["xml"] == name)
            .unwrap();
        child(&repo, &output, entry);
        return;
    }
    let mut counts = Vec::new();
    for entry in index["builds"].as_array().unwrap() {
        let name = entry["xml"].as_str().unwrap();
        let stdout = fs::File::create(output.join(format!("{name}.stdout.log"))).unwrap();
        let stderr = fs::File::create(output.join(format!("{name}.stderr.log"))).unwrap();
        let mut process = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--nocapture"])
            .env(CHILD, name)
            .env(OUTPUT, &output)
            .current_dir(repo.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .unwrap();
        let start = Instant::now();
        let status = loop {
            if let Some(status) = process.try_wait().unwrap() {
                break status;
            }
            if start.elapsed() > Duration::from_secs(300) {
                process.kill().unwrap();
                let _ = process.wait();
                panic!("item assembly original child timeout: {name}");
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        assert!(
            status.success(),
            "item assembly child {name} failed; inspect {}",
            output.display()
        );
        counts.push(json!({"xml":name,"exit":status.code()}));
    }
    fs::write(
        output.join("summary.json"),
        serde_json::to_vec_pretty(
            &json!({"children":counts,"fresh_original_hosts":10,"full_native_builds":0}),
        )
        .unwrap(),
    )
    .unwrap();
}
