//! Ordered item-set component oracle inside unchanged complete original Load.
//! Native inventory and activation completion are separate admission claims.
#[path = "item_assembly_graph.rs"]
mod graph;
#[allow(dead_code)]
#[path = "configuration_preparation_source.rs"]
mod source;
use mlua::{Function, Lua, LuaSerdeExt, Table, Value};
use poe_optimizer_data::item_assembly::ItemInventoryPolicy;
use poe_optimizer_engine::source_program::{
    ProgramTableId as Id, ProgramValue as V, ProgramValueGraph as Graph,
};
use poe_optimizer_import::{
    item_loading::assembly::{AssemblyError, AssemblyErrorKind},
    item_sets::{
        FinishLoadInput, ItemSetInput, ItemSetLimits, ItemSetPhase, ItemSetState, LegacySlotInput,
        SetRuneInput, SetSlotInput, SocketUrlInput, TradeWeightInput,
    },
    item_source::{self, ItemSourceNode},
    source_xml::PobContentEntry,
};
use poe_optimizer_pob::runtime::RuntimeError;
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    rc::Rc,
    time::{Duration, Instant},
};
const OBSERVER: &str = include_str!("item_set_lifecycle.lua");
#[derive(Debug, Clone)]
pub struct Case {
    pub label: &'static str,
    pub xml: String,
    pub structural: bool,
    pub source_error: bool,
    pub loads: usize,
}
/// All Item byte ranges and unrelated root sections are retained unchanged.
/// Only the named derived set/legacy instructions replace the original Items
/// metadata. The repeated case adds a second Items container without new items.
pub fn cases(original: &str, derived: bool) -> Vec<Case> {
    let mut cases = vec![Case {
        label: "original",
        xml: original.into(),
        structural: false,
        source_error: false,
        loads: 1,
    }];
    if !derived {
        return cases;
    }
    let document = roxmltree::Document::parse(original).unwrap();
    let containers = document
        .root_element()
        .children()
        .filter(|n| n.has_tag_name("Items"))
        .collect::<Vec<_>>();
    assert_eq!(containers.len(), 1);
    let container = containers[0];
    let item_bytes = container
        .children()
        .filter(|n| n.has_tag_name("Item"))
        .map(|n| &original[n.range()])
        .collect::<Vec<_>>()
        .join("\n");
    let inputs = [
        (
            "legacy_only",
            r#"useSecondWeaponSet="true" showStatDifferences="false""#,
            r#"<Slot name="Ring 1" itemId="1"/><Slot name="Flask 1" itemId="nonnumeric" active="true"/><Slot name="Unknown slot" itemId="9" active="true"/><Slot name="Flask 1" itemId="0" active="false"/>"#,
            false,
        ),
        (
            "missing_and_duplicate_ids",
            r#"activeItemSet="999""#,
            r#"<ItemSet id="3" title="First"/><ItemSet title="Allocated one"/><ItemSet id="1.0" title="Replacement one"/><ItemSet id="nonnumeric" title="Allocated two"/><ItemSet id="2.5" title="Fractional"/>"#,
            false,
        ),
        (
            "mixed_rows",
            r#"activeItemSet="2""#,
            r#"<ItemSet id="2" title="Mixed" useSecondWeaponSet="nil"><Slot name="Ring 1" itemId="1" active="TRUE" note="first"/><Slot name="Ring 1" itemId="" active="true" itemPbURL="" note=""/><Slot name="Unknown slot" itemId="99"/><Slot name="useSecondWeaponSet" itemId="99"/><RuneSlot slotName="Helmet Rune #1" runeName="Unknown rune"/><RuneSlot slotName="Helmet Rune #1" runeName="None"/><RuneSlot slotName="Unknown rune slot" runeName="None"/><SocketIdURL nodeId="9" itemPbURL="first"/><SocketIdURL nodeId="9.0" itemPbURL="last"/></ItemSet>"#,
            false,
        ),
        (
            "duplicate_numeric_set_winner",
            r#"activeItemSet="2""#,
            r#"<ItemSet id="2" title="Old"><Slot name="Ring 1" itemId="1" note="old"/></ItemSet><ItemSet id="2.0" title="Winner"><Slot name="Ring 2" itemId="2" note="new"/></ItemSet>"#,
            false,
        ),
        (
            "invalid_socket_key",
            r#"activeItemSet="2""#,
            r#"<ItemSet id="2" title="Partial"><Slot name="Ring 1" itemId="1" note="retained prefix"/><SocketIdURL nodeId="nonnumeric" itemPbURL="never stored"/></ItemSet><ItemSet id="3" title="Not reached"/>"#,
            true,
        ),
        (
            "missing_socket_key",
            r#"activeItemSet="2""#,
            r#"<ItemSet id="2" title="Partial"><SocketIdURL itemPbURL="never stored"/></ItemSet>"#,
            true,
        ),
        (
            "slot_scalar_receiver",
            r#"activeItemSet="2""#,
            r#"<ItemSet id="2" title="Partial"><Slot name="id" itemId="1"/></ItemSet>"#,
            true,
        ),
        (
            "trade_ordinary_text",
            r#"activeItemSet="2""#,
            r#"<TradeSearchWeights><Stat weightMult="2"/>text receiver<Stat stat="unknown"/></TradeSearchWeights><ItemSet id="2" title="Not reached"/>"#,
            true,
        ),
        (
            "trade_cdata_text",
            r#"activeItemSet="2""#,
            r#"<TradeSearchWeights><Stat weightMult="2"/><![CDATA[text receiver]]><Stat stat="unknown"/></TradeSearchWeights><ItemSet id="2" title="Not reached"/>"#,
            true,
        ),
        (
            "rune_scalar_receiver",
            r#"activeItemSet="2""#,
            r#"<ItemSet id="2" title="Partial"><RuneSlot slotName="title" runeName="None"/></ItemSet>"#,
            true,
        ),
    ];
    for (label, attrs, body, source_error) in inputs {
        let replacement = format!("<Items {attrs}>\n{item_bytes}\n{body}\n</Items>");
        let xml = format!(
            "{}{}{}",
            &original[..container.range().start],
            replacement,
            &original[container.range().end..]
        );
        cases.push(Case {
            label,
            xml,
            structural: true,
            source_error,
            loads: 1,
        });
    }
    let repeated = format!(
        "{}{}\n<Items activeItemSet=\"4\" showStatDifferences=\"false\"><Slot name=\"Ring 1\" itemId=\"0\"/><ItemSet id=\"4\" title=\"Second load\"><Slot name=\"Ring 2\" itemId=\"1\" note=\"second\"/></ItemSet></Items>{}",
        &original[..container.range().start],
        &original[container.range()],
        &original[container.range().end..]
    );
    cases.push(Case {
        label: "repeated_items_requires_activation_continuation",
        xml: repeated,
        structural: true,
        source_error: false,
        loads: 2,
    });
    cases
}

pub struct SourceObservation {
    pub lua: Lua,
    pub report: Table,
    pub outcome: Result<serde_json::Value, RuntimeError>,
}
pub fn observe(repo: &Path, directory: &Path, case: &Case) -> SourceObservation {
    fs::create_dir_all(directory).unwrap();
    let host = Rc::new(RefCell::new(None::<Lua>));
    let module = Rc::new(RefCell::new(None::<Table>));
    let capture = Rc::new(RefCell::new(None::<Table>));
    let before_source = |lua: &Lua| -> Result<(), RuntimeError> {
        *host.borrow_mut() = Some(lua.clone());
        *module.borrow_mut() = Some(
            lua.load(OBSERVER)
                .set_name("@item_set_lifecycle.lua")
                .eval()?,
        );
        Ok(())
    };
    let before_build = |_lua: &Lua| -> Result<Function, RuntimeError> {
        let active: Table = module
            .borrow()
            .as_ref()
            .unwrap()
            .raw_get::<Function>("start")?
            .call(true)?;
        let finish = active.raw_get("finish")?;
        *capture.borrow_mut() = Some(active);
        Ok(finish)
    };
    let outcome = source::observe_with_build_hook_unwrapped(
        &repo.join("vendor/path-of-building-poe2"),
        directory,
        &case.xml,
        None,
        case.structural,
        Some(&before_source),
        Some(&before_build),
        None,
    );
    let lua = host.borrow().as_ref().unwrap().clone();
    let report = capture
        .borrow()
        .as_ref()
        .unwrap()
        .raw_get("report")
        .expect("observer must finish even when original Load fails");
    assert!(matches!(
        lua.load("return debug.gethook()").eval::<Value>().unwrap(),
        Value::Nil
    ));
    SourceObservation {
        lua,
        report,
        outcome,
    }
}
pub fn canonical(value: Table) -> serde_json::Value {
    graph::canonical(&graph::capture(&[Value::Table(value)]).unwrap()).unwrap()
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn id(value: &V) -> Id {
    match value {
        V::Table(id) => *id,
        _ => panic!("expected diagnostic table"),
    }
}
fn field(g: &Graph, table: Id, name: &str) -> V {
    g.tables[table.0 as usize - 1]
        .entries
        .iter()
        .find_map(|(k, v)| matches!(k,V::Bytes(k) if k==name.as_bytes()).then(|| v.clone()))
        .unwrap_or(V::Nil)
}
fn keep(g: &mut Graph, table: Id, names: &[&str]) {
    g.tables[table.0 as usize - 1]
        .entries
        .retain(|(k, _)| matches!(k,V::Bytes(k) if names.iter().any(|name|k==name.as_bytes())));
}
fn source_graph(table: Table) -> Graph {
    graph::capture(&[Value::Table(table)]).unwrap()
}
/// Fixed diagnostic projection; never imported into either producer. Filtering
/// in one graph preserves set/active/prior and live-child identity relationships.
/// Constructor trade and error-only prior are absent from the original witness.
pub fn common_graph(
    mut g: Graph,
    source_runes: bool,
    constructor: bool,
    error_prefix: bool,
) -> Graph {
    graph::canonical(&g).unwrap();
    assert_eq!(g.values.len(), 1);
    let root = id(&g.values[0]);
    let mut names = vec![
        "itemSets",
        "itemSetOrderList",
        "activeItemSet",
        "activeItemSetId",
        "slots",
        "runeSlots",
        "showStatDifferences",
    ];
    if !error_prefix {
        names.push("previousActiveItemSet");
    }
    if !constructor {
        names.push("trade");
    }
    keep(&mut g, root, &names);
    let slots = id(&field(&g, root, "slots"));
    let rows = g.tables[slots.0 as usize - 1]
        .entries
        .iter()
        .map(|(_, v)| id(v))
        .collect::<BTreeSet<_>>();
    for slot in rows {
        keep(
            &mut g,
            slot,
            &[
                "slotName",
                "nodeId",
                "selItemId",
                "active",
                "inactive",
                "note",
                "activate",
                "jewelSocketList",
            ],
        );
        if let V::Table(activate) = field(&g, slot, "activate") {
            keep(&mut g, activate, &["state"]);
        }
    }
    let runes = id(&field(&g, root, "runeSlots"));
    let rows = g.tables[runes.0 as usize - 1]
        .entries
        .iter()
        .map(|(_, v)| id(v))
        .collect::<BTreeSet<_>>();
    for row in rows {
        if source_runes {
            let index = match field(&g, row, "selIndex") {
                V::Number(n) if n.is_finite() && n >= 1.0 && n.fract() == 0.0 => n,
                _ => panic!("rune selected index"),
            };
            let list = id(&field(&g, row, "list"));
            let entries = &g.tables[list.0 as usize - 1].entries;
            let mut indices = BTreeSet::new();
            for (key, _) in entries {
                let V::Number(n) = key else {
                    panic!("rune list key")
                };
                assert!(
                    n.is_finite() && *n >= 1.0 && n.fract() == 0.0 && *n <= entries.len() as f64
                );
                assert!(indices.insert(*n as usize));
            }
            assert_eq!(indices.len(), entries.len());
            let selected = entries
                .iter()
                .find(|(k, _)| matches!(k,V::Number(n) if *n==index))
                .expect("rune selected row");
            let name = field(&g, id(&selected.1), "name");
            assert!(matches!(name, V::Bytes(_)));
            g.tables[row.0 as usize - 1].entries =
                vec![(V::Bytes(b"selected_name".to_vec()), name)];
        } else {
            keep(&mut g, row, &["selected_name"]);
        }
    }
    g
}
pub fn common(value: Graph, source_runes: bool, constructor: bool, error_prefix: bool) -> Json {
    graph::canonical(&common_graph(
        value,
        source_runes,
        constructor,
        error_prefix,
    ))
    .unwrap()
}
/// Source errors cannot absorb host, conversion, stack, memory or observer errors.
pub fn lua_source_error(error: &mlua::Error) -> bool {
    match error {
        mlua::Error::RuntimeError(message) => !message.contains("item-set lifecycle:"),
        mlua::Error::CallbackError { cause, .. } => lua_source_error(cause),
        _ => false,
    }
}
fn source_error(error: &RuntimeError) -> bool {
    matches!(error,RuntimeError::Lua(error) if lua_source_error(error))
}
fn attr<'a>(node: &'a ItemSourceNode<'_>, name: &str) -> Option<&'a str> {
    node.element().attribute(name).map(|v| v.decoded())
}
struct NativeObservation {
    state: ItemSetState,
    constructor: Graph,
    trace: Vec<Json>,
    result: Result<(), AssemblyError>,
    omitted_item_occurrences: usize,
    remaining_containers: usize,
}
/// XML strings are inputs. The original output graph is never supplied here.
/// Item children are deliberately outside this component lane; the independently
/// produced inventory lane is a separate root-owned integration test.
fn native(policy: &ItemInventoryPolicy, xml: &str) -> NativeObservation {
    let projection = item_source::project_xml(xml).unwrap();
    let container = projection.containers().first().expect("Items container");
    let mut state = ItemSetState::new(policy, ItemSetLimits::default()).unwrap();
    let constructor = state.snapshot().unwrap();
    let mut trace = Vec::new();
    let mut omitted = 0;
    let result = (|| {
        state.begin_load()?;
        for (ordinal, entry) in container.ordered_content().consumed().iter().enumerate() {
            let PobContentEntry::Element { child_index } = entry else {
                trace.push(json!({"stage":"ignored_Items_text","container":0,"ordinal":ordinal,"entry":entry}));
                continue;
            };
            let node = &container.children()[*child_index];
            assert!(
                !node.element().has_namespaces(),
                "fixture namespace context unsupported"
            );
            let record = |stage: &str, n: &ItemSourceNode<'_>| json!({"stage":stage,"container":0,"ordinal":ordinal,"range":n.element().source_range(),"element":n.element().name()});
            match node.element().name() {
                "Item" => {
                    omitted += 1;
                    trace.push(record("source_only_inventory_item", node));
                }
                "Slot" => {
                    trace.push(record("legacy_slot", node));
                    state.legacy_slot(LegacySlotInput {
                        name: attr(node, "name"),
                        item_id: attr(node, "itemId"),
                        active: attr(node, "active"),
                    })?;
                }
                "ItemSet" => {
                    trace.push(record("begin_item_set", node));
                    state.begin_item_set(ItemSetInput {
                        id: attr(node, "id"),
                        title: attr(node, "title"),
                        use_second_weapon_set: attr(node, "useSecondWeaponSet"),
                    })?;
                    for (child_ordinal, entry) in
                        node.ordered_content().consumed().iter().enumerate()
                    {
                        let PobContentEntry::Element { child_index } = entry else {
                            trace.push(json!({"stage":"ignored_ItemSet_text","container":0,"ordinal":ordinal,"child_ordinal":child_ordinal,"entry":entry}));
                            continue;
                        };
                        let child = &node.children()[*child_index];
                        assert!(
                            !child.element().has_namespaces(),
                            "fixture namespace context unsupported"
                        );
                        let mut event = record(child.element().name(), child);
                        event["child_ordinal"] = child_ordinal.into();
                        trace.push(event);
                        match child.element().name() {
                            "Slot" => state.set_slot(SetSlotInput {
                                name: attr(child, "name"),
                                item_id: attr(child, "itemId"),
                                active: attr(child, "active"),
                                item_pb_url: attr(child, "itemPbURL"),
                                note: attr(child, "note"),
                            })?,
                            "RuneSlot" => state.set_rune(SetRuneInput {
                                slot_name: attr(child, "slotName"),
                                rune_name: attr(child, "runeName"),
                            })?,
                            "SocketIdURL" => state.socket_url(SocketUrlInput {
                                node_id: attr(child, "nodeId"),
                                item_pb_url: attr(child, "itemPbURL"),
                            })?,
                            _ => {}
                        }
                    }
                    trace.push(record("finish_item_set", node));
                    state.finish_item_set()?;
                }
                "TradeSearchWeights" => {
                    for (child_ordinal, entry) in
                        node.ordered_content().consumed().iter().enumerate()
                    {
                        let PobContentEntry::Element { child_index } = entry else {
                            trace.push(json!({"stage":"trade_text","container":0,"ordinal":ordinal,"child_ordinal":child_ordinal,"entry":entry}));
                            state.trade_text()?;
                            unreachable!("consumed text must fail before attributes");
                        };
                        let child = &node.children()[*child_index];
                        assert!(
                            !child.element().has_namespaces(),
                            "fixture namespace context unsupported"
                        );
                        let mut event = record("trade_weight", child);
                        event["child_ordinal"] = child_ordinal.into();
                        trace.push(event);
                        state.trade_weight(TradeWeightInput {
                            label: attr(child, "label"),
                            stat: attr(child, "stat"),
                            weight_mult: attr(child, "weightMult"),
                        })?;
                    }
                }
                _ => trace.push(record("ignored_direct_child", node)),
            }
        }
        trace.push(json!({"stage":"finish_load","range":container.element().source_range()}));
        state.finish_load(FinishLoadInput {
            active_item_set: attr(container, "activeItemSet"),
            use_second_weapon_set: attr(container, "useSecondWeaponSet"),
            show_stat_differences: attr(container, "showStatDifferences"),
        })
    })();
    NativeObservation {
        state,
        constructor,
        trace,
        result,
        omitted_item_occurrences: omitted,
        remaining_containers: projection.containers().len() - 1,
    }
}
fn receipt(observation: &SourceObservation) -> Json {
    let mut report = serde_json::Map::new();
    for key in [
        "events",
        "functions",
        "scope",
        "bounds",
        "retained_objects",
        "rows",
        "text_bytes",
        "incomplete_calls",
    ] {
        report.insert(
            key.into(),
            observation
                .lua
                .from_value::<Json>(observation.report.raw_get(key).unwrap())
                .unwrap(),
        );
    }
    let mut states = Vec::new();
    for row in observation
        .report
        .raw_get::<Table>("states")
        .unwrap()
        .sequence_values::<Table>()
    {
        let row = row.unwrap();
        let mut entry = serde_json::Map::new();
        for key in [
            "event_ordinal",
            "call_ordinal",
            "phase",
            "name",
            "receiver_token",
            "prior_set_token",
        ] {
            entry.insert(
                key.into(),
                observation
                    .lua
                    .from_value::<Json>(row.raw_get(key).unwrap())
                    .unwrap(),
            );
        }
        entry.insert("graph".into(), canonical(row.raw_get("value").unwrap()));
        states.push(Json::Object(entry));
    }
    report.insert("states".into(), states.into());
    report.insert(
        "finite_post_import_graph".into(),
        canonical(observation.report.raw_get("finite_post_import").unwrap()),
    );
    report.insert(
        "outcome".into(),
        match &observation.outcome {
            Ok(value) => json!({"ok":true,"source_report":value}),
            Err(error) => {
                json!({"ok":false,"source_error":source_error(error),"error":error.to_string()})
            }
        },
    );
    Json::Object(report)
}
fn states(observation: &SourceObservation, name: &str, phase: &str) -> Vec<Table> {
    observation
        .report
        .raw_get::<Table>("states")
        .unwrap()
        .sequence_values::<Table>()
        .map(Result::unwrap)
        .filter(|row| {
            row.raw_get::<String>("name").unwrap() == name
                && row.raw_get::<String>("phase").unwrap() == phase
        })
        .collect()
}
fn save(path: &Path, value: &Json) {
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
const SCOPE: &str = "source-fed set-materialization component at the first original SetActiveItemSet entry; complete mixed set rows, numeric winners/order and active/prior aliases; selected live fields and child aliases; rune selected names only. Excludes inventory production, dropdown arrays/indices/effect data, slot parent/number/weapon fields omitted by the observer, exact trade-transform Function identity, activation, PopulateSlots, SyncLoadouts, trailing flags and ResetUndo. Error snapshots omit prior because the original final-state witness has no prior argument; constructor comparison omits uninitialized source trade storage.";

fn compare_case(repo: &Path, directory: &Path, policy: &ItemInventoryPolicy, case: &Case) -> Json {
    fs::create_dir_all(directory).unwrap();
    fs::write(directory.join("input.xml"), &case.xml).unwrap();
    let observation = observe(repo, &directory.join("host"), case);
    let source = receipt(&observation);
    save(&directory.join("source.json"), &source);
    // The observer retains/rechecks actual Function identity; declaration and
    // file evidence additionally bind the expected pinned role.
    let function = source["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "items_load")
        .unwrap();
    assert_eq!(function["first_line"], 1193);
    assert_eq!(function["last_line"], 1320);
    assert!(
        function["source"]
            .as_str()
            .unwrap()
            .replace('\\', "/")
            .ends_with("Classes/ItemsTab.lua")
    );
    let mut native = native(policy, &case.xml);
    let source_before = states(&observation, "items_load", "before");
    assert!(!source_before.is_empty());
    let source_constructor = common(
        source_graph(source_before[0].raw_get("value").unwrap()),
        true,
        true,
        false,
    );
    let native_constructor = common(native.constructor.clone(), false, true, false);
    let constructor_equal = source_constructor == native_constructor;
    let source_entry = states(&observation, "activate_set", "before");
    let native_graph = native.state.snapshot().unwrap();
    let activation_join = if case.source_error {
        assert!(native.state.pending_activation().is_none());
        Json::Null
    } else {
        assert!(
            !source_entry.is_empty(),
            "expected original activation entry; inspect source.json"
        );
        let load_ordinal: u64 = source_before[0].raw_get("call_ordinal").unwrap();
        let activation_ordinal: u64 = source_entry[0].raw_get("call_ordinal").unwrap();
        let events = source["events"].as_array().unwrap();
        let call = events
            .iter()
            .find(|event| event["ordinal"] == activation_ordinal)
            .unwrap();
        assert_eq!(call["event"], "call");
        assert_eq!(call["name"], "activate_set");
        assert_eq!(call["load_call_ordinal"], load_ordinal);
        assert_eq!(call["parent_call_ordinal"], load_ordinal);
        let load_receiver: u64 = source_before[0].raw_get("receiver_token").unwrap();
        assert_eq!(call["receiver_token"], load_receiver);
        // Read the original number from its retained Lua descriptor so JSON
        // formatting cannot normalize a signed-zero selector before comparison.
        let actual_call: Table = observation
            .report
            .raw_get::<Table>("events")
            .unwrap()
            .raw_get(activation_ordinal)
            .unwrap();
        let requested: Table = actual_call.raw_get("requested_set").unwrap();
        assert_eq!(requested.raw_get::<String>("kind").unwrap(), "number");
        let source_number: f64 = requested.raw_get("value").unwrap();
        let native_number = native.state.pending_activation().unwrap().value().unwrap();
        assert_eq!(source_number.to_bits(), native_number.to_bits());
        json!({"first_load_call_ordinal":load_ordinal,"activation_call_ordinal":activation_ordinal,
            "direct_parent_equal":true,"load_receiver_equal":true,"requested_set_bits_equal":true,
            "source_requested_set_bits":format!("{:016x}",source_number.to_bits()),
            "native_requested_set_bits":format!("{:016x}",native_number.to_bits()),
            "actual_argument_arity_claim":false})
    };
    let source_prefix = if case.source_error {
        common(
            source_graph(observation.report.raw_get("finite_post_import").unwrap()),
            true,
            false,
            true,
        )
    } else {
        assert!(
            !source_entry.is_empty(),
            "expected original activation entry; inspect source.json"
        );
        common(
            source_graph(source_entry[0].raw_get("value").unwrap()),
            true,
            false,
            false,
        )
    };
    let native_prefix = common(native_graph.clone(), false, false, case.source_error);
    let prefix_equal = source_prefix == native_prefix;
    let error = native
        .result
        .as_ref()
        .err()
        .map(|e| json!({"kind":format!("{:?}",e.kind),"message":e.message}));
    let native_phase = native.state.phase();
    let continuation = serde_json::to_value(native.state.continuation()).unwrap();
    let mut repeat_blocked = None;
    if case.loads > 1 && native.result.is_ok() {
        let before = graph::canonical(&native_graph).unwrap();
        let blocked = native
            .state
            .begin_load()
            .expect_err("second Load must not bypass native activation");
        let same = before == graph::canonical(&native.state.snapshot().unwrap()).unwrap();
        repeat_blocked = Some(
            json!({"kind":format!("{:?}",blocked.kind),"message":blocked.message,"state_unchanged":same}),
        );
        assert_eq!(blocked.kind, AssemblyErrorKind::Unsupported);
        assert!(same);
    }
    let events = source["events"].as_array().unwrap();
    let load_calls = events
        .iter()
        .filter(|e| e["name"] == "items_load" && e["event"] == "call")
        .count();
    let load_returns = events
        .iter()
        .filter(|e| e["name"] == "items_load" && e["event"] == "return")
        .count();
    let summary = json!({"label":case.label,"input_sha256":hash(case.xml.as_bytes()),"structurally_derived":case.structural,"scope":SCOPE,
        "source_load_calls":load_calls,"source_load_returns":load_returns,"source_activation_entries":source_entry.len(),"activation_join":activation_join,
        "constructor_equal":constructor_equal,"materialization_graph_equal":prefix_equal,
        "source_constructor":source_constructor,"native_constructor":native_constructor,"source_prefix":source_prefix,"native_prefix":native_prefix,
        "native_snapshot":graph::canonical(&native_graph).unwrap(),"native_phase":native_phase,"native_error":error,"native_trace":native.trace,"native_usage":native.state.usage(),"native_continuation":continuation,
        "source_only_item_occurrences":native.omitted_item_occurrences,"remaining_native_containers":native.remaining_containers,"repeated_native_block":repeat_blocked,
        "native_inventory_claim":false,"native_whole_load_claim":false,"source_error_expected":case.source_error,"error_location_claim":"native trace retains exact XML occurrence/stage; source whole-Load exception is retained separately, not an exact instruction interception"});
    save(&directory.join("comparison.json"), &summary);
    assert!(
        constructor_equal,
        "fresh constructor context mismatch: {}",
        directory.display()
    );
    assert!(
        prefix_equal,
        "set materialization graph mismatch: {}",
        directory.display()
    );
    if case.source_error {
        assert!(
            observation.outcome.as_ref().is_err_and(source_error),
            "not a source RuntimeError; inspect receipt"
        );
        assert_eq!(native.result.unwrap_err().kind, AssemblyErrorKind::Source);
        assert_eq!(native_phase, ItemSetPhase::Failed);
        assert_eq!(load_calls, 1);
        assert_eq!(load_returns, 0);
        assert!(source_entry.is_empty());
    } else {
        observation.outcome.unwrap();
        native.result.unwrap();
        assert_eq!(native_phase, ItemSetPhase::AwaitingActivation);
        assert_eq!(load_calls, case.loads);
        assert_eq!(load_returns, case.loads);
        assert_eq!(
            observation
                .report
                .raw_get::<Table>("incomplete_calls")
                .unwrap()
                .raw_len(),
            0
        );
    }
    summary
}
const TEST: &str = "all_five_original_set_materialization_and_derived_histories";
const CHILD: &str = "POE_ITEM_SET_MATERIALIZATION_CHILD";
const OUTPUT: &str = "POE_ITEM_SET_MATERIALIZATION_OUTPUT";
pub fn run() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let input = repo.join("tests/fixtures/builds/breadth-20260908");
    let index: Json = serde_json::from_slice(&fs::read(input.join("index.json")).unwrap()).unwrap();
    let output = std::env::var_os(OUTPUT)
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("runs/r2ae-item-set-loading-01/source"));
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
        let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
        let directory = output.join(&name);
        fs::create_dir_all(&directory).unwrap();
        let mut summaries = Vec::new();
        for case in cases(&xml, name == index["builds"][0]["xml"].as_str().unwrap()) {
            summaries.push(compare_case(
                &repo,
                &directory.join(case.label),
                &snapshot.item_assembly().policy().inventory,
                &case,
            ));
        }
        let report = json!({"xml":name,"xml_sha256":entry["xml_sha256"],"bundled_package_sha256":poe_optimizer_data::game_data::bundled_package_sha256(),"original_items_tab_sha256":hash(&fs::read(repo.join("vendor/path-of-building-poe2/src/Classes/ItemsTab.lua")).unwrap()),"observer_sha256":hash(OBSERVER.as_bytes()),"cases":summaries,"scope":SCOPE,"source_methods_replaced":false,"source_output_imported_as_native_state":false,"control_scope":"unchanged shared observer has separate whole-Load controls; this target does not claim new universal observer noninterference"});
        save(&output.join(format!("{name}.json")), &report);
        return;
    }
    let mut children = Vec::new();
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
            if start.elapsed() > Duration::from_secs(600) {
                process.kill().unwrap();
                let _ = process.wait();
                panic!("item-set materialization child timeout {name}");
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        assert!(
            status.success(),
            "item-set materialization child failed {name}; inspect {}",
            output.display()
        );
        children.push(json!({"xml":name,"exit":status.code()}));
    }
    save(
        &output.join("summary.json"),
        &json!({"children":children,"original_inputs":5,"derived_cases_on_first_original":11,"fresh_source_hosts":16,"scope":SCOPE,"native_whole_load_claim":false,"native_inventory_claim":false}),
    );
}
