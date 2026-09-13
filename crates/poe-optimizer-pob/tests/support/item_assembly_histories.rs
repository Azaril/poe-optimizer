//! Directed original-method histories after a complete original build import.
//! These specimens are declared inputs, not replacements for the five saved builds.
use super::{FIELDS, compare_graph, dependencies};
#[path = "item_assembly_local_histories.rs"]
mod local;
use mlua::{Function, Lua, MultiValue, Table, Value};
use poe_optimizer_data::game_data::GameDataSnapshot;
use poe_optimizer_import::item_loading::*;
use serde_json::{Value as Json, json};
use std::collections::BTreeMap;
const INPUT_FIELDS: &[&str] = &[
    "base",
    "itemSocketCount",
    "variant",
    "variantList",
    "versionList",
    "selectedVersion",
    "variantGroups",
    "variantGroupSelections",
    "allowDuplicateVariants",
    "hasAltVariant",
    "hasAltVariant2",
    "hasAltVariant3",
    "hasAltVariant4",
    "hasAltVariant5",
    "variantAlt",
    "variantAlt2",
    "variantAlt3",
    "variantAlt4",
    "variantAlt5",
];
fn observe<T>(
    lua: &Lua,
    module: &Table,
    action: impl FnOnce() -> mlua::Result<T>,
) -> (mlua::Result<T>, Table) {
    let fields = lua
        .create_sequence_from(FIELDS.iter().chain(INPUT_FIELDS.iter()).copied())
        .unwrap();
    let capture: Table = module
        .raw_get::<Function>("start")
        .unwrap()
        .call(fields)
        .unwrap();
    struct Cleanup(Option<Function>);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            if let Some(finish) = self.0.take() {
                let _ = finish.call::<()>(());
            }
        }
    }
    let mut cleanup = Cleanup(Some(capture.raw_get::<Function>("finish").unwrap()));
    let result = action();
    // Remove the hook on Lua error and during a Rust unwind.
    let report: Table = cleanup.0.take().unwrap().call(()).unwrap();
    (result, report)
}
fn last(report: &Table) -> Table {
    let events: Table = report.raw_get("events").unwrap();
    assert!(events.raw_len() > 0);
    events.raw_get(events.raw_len()).unwrap()
}
fn item(lua: &Lua) -> Table {
    let item: Table = lua
        .globals()
        .raw_get::<Function>("new")
        .unwrap()
        .call("Item")
        .unwrap();
    item.get::<Function>("Item")
        .unwrap()
        .call::<Table>((item.clone(), ""))
        .unwrap();
    item.raw_set("id", 73).unwrap();
    item
}
fn provider<'a>(
    snapshot: &'a GameDataSnapshot,
    parser: &Function,
) -> dependencies::Recorded<NativeItemLoadProvider<'a, dependencies::OriginalParser>> {
    dependencies::Recorded {
        inner: NativeItemLoadProvider::with_native_assembly(
            snapshot,
            dependencies::OriginalParser {
                function: parser.clone(),
                calls: 0,
            },
        ),
        requests: Vec::new(),
    }
}
fn machine(snapshot: &GameDataSnapshot) -> ItemLoadMachine<'_> {
    let mut m = ItemLoadMachine::new(snapshot.item_loading());
    m.set_xml_attributes(&BTreeMap::from([("id".into(), "73".into())]));
    m
}
fn completed(m: &ItemLoadMachine<'_>, event: &Table, label: &str) -> Json {
    assert_eq!(
        m.status(),
        ItemLoadStatus::Complete,
        "{label}: {:?}",
        m.pending()
    );
    assert!(event.raw_get::<bool>("completed").unwrap(), "{label}");
    let graph = m
        .assembly_progress()
        .expect("complete native producer must own its graph");
    assert!(graph.is_complete());
    compare_graph(graph, event, label)
}
#[allow(clippy::too_many_arguments)]
fn parse_step(
    lua: &Lua,
    module: &Table,
    source: &Table,
    parse: &Function,
    m: &mut ItemLoadMachine<'_>,
    p: &mut impl ItemLoadProvider,
    raw: &str,
    label: &str,
) -> (Json, Table) {
    let (result, report) = observe(lua, module, || parse.call::<()>((source.clone(), raw)));
    result.unwrap();
    m.apply_text(raw, p).unwrap();
    let event = last(&report);
    (completed(m, &event, label), event)
}
#[allow(clippy::too_many_arguments)]
fn finish_step(
    lua: &Lua,
    module: &Table,
    source: &Table,
    build: &Function,
    m: &mut ItemLoadMachine<'_>,
    p: &mut impl ItemLoadProvider,
    label: &str,
) -> Json {
    let (result, report) = observe(lua, module, || build.call::<()>(source.clone()));
    result.unwrap();
    m.finish_load(p).unwrap();
    assert!(m.assembled().is_some());
    completed(m, &last(&report), label)
}
pub(super) fn run(
    lua: &Lua,
    module: &Table,
    parser: &Function,
    snapshot: &GameDataSnapshot,
) -> Json {
    let class: Table = lua
        .globals()
        .raw_get::<Table>("common")
        .unwrap()
        .raw_get::<Table>("classes")
        .unwrap()
        .raw_get("Item")
        .unwrap();
    let parse: Function = class.raw_get("ParseRaw").unwrap();
    let build: Function = class.raw_get("BuildModList").unwrap();
    let mut variants = Vec::new();
    for (index,headers) in [
        "Variant: First\nVariant: Second\nSelected Variant: 0",
        "Variant: First\nVariant: Second\nSelected Variant: -2",
        "Variant: First\nVariant: Second\nSelected Variant: 99",
        "Variant: First\nVariant: Second\nSelected Variant: 1\nHas Alt Variant: true\nSelected Alt Variant: 1\nAllow Duplicate Variants: true",
        "Version: Old\nVersion: New\nVariant: First\nVariant: Second\nSelected Version: 0\nSelected Variant: 99",
        "Version: Old\nVersion: New\nVariant: First\nVariant: Second\nSelected Version: 2\nSelected Variant Group: 1=2\nSelected Variant Group: 2=2",
    ].iter().enumerate() {
        let mods = if headers.contains("Selected Variant Group") { "{version:1}{variant:1}{group:1}+10 to maximum Life\n{version:2}{variant:2}{group:1,2}+20 to maximum Life\n{variant:1}+5 to Strength" }
            else if headers.contains("Version:") { "{version:1}{variant:1}+10 to maximum Life\n{version:2}{variant:2}+20 to maximum Life\n{variant:1}+5 to Strength" }
            else { "{variant:1}+10 to maximum Life\n{variant:2}+20 to maximum Life\n{variant:1}+5 to Strength" };
        let raw=format!("Rarity: Rare\nDirected accessory\nGold Ring\n{headers}\nImplicits: 0\n{mods}");
        let source=item(lua); let mut native=machine(snapshot); let mut p=provider(snapshot,parser);
        let (parsed,_)=parse_step(lua,module,&source,&parse,&mut native,&mut p,&raw,&format!("variant {index} ParseRaw"));
        let finished=finish_step(lua,module,&source,&build,&mut native,&mut p,&format!("variant {index} final"));
        variants.push(json!({"headers":headers,"parse":parsed,"final":finished}));
    }
    let source = item(lua);
    let mut native = machine(snapshot);
    let mut p = provider(snapshot, parser);
    let mut reuse = Vec::new();
    let first = "Rarity: Rare\nEarlier accessory\nGold Ring\nItem Level: 60\nQuality: 17\nCorrupted\nImplicits: 0\n+(10-20) to maximum Life";
    let second = "Rarity: Normal\nGold Ring\nImplicits: 0\n+(20-40) to maximum Life";
    for (index, raw, range) in [(0, first, 0.125), (1, second, 0.875), (2, first, 0.5)] {
        let (parsed, _) = parse_step(
            lua,
            module,
            &source,
            &parse,
            &mut native,
            &mut p,
            raw,
            &format!("reuse {index} ParseRaw"),
        );
        // This fixture has only one explicit row: the same raw assignment made by
        // original ItemsTab.Load's legacy ModRange loop, then the original method.
        for group in ["buffModLines", "enchantModLines", "implicitModLines"] {
            assert_eq!(source.raw_get::<Table>(group).unwrap().raw_len(), 0);
        }
        let rows: Table = source.raw_get("explicitModLines").unwrap();
        assert_eq!(rows.raw_len(), 1);
        rows.raw_get::<Table>(1)
            .unwrap()
            .raw_set("range", range)
            .unwrap();
        native
            .apply_mod_range(Some("1"), Some(&range.to_string()))
            .unwrap();
        let finished = finish_step(
            lua,
            module,
            &source,
            &build,
            &mut native,
            &mut p,
            &format!("reuse {index} range/final"),
        );
        reuse.push(json!({"range":range,"parse":parsed,"final":finished}));
    }
    let retained = native.assembled().unwrap().clone();
    let stable = super::observation::canonical(&retained.snapshot().unwrap()).unwrap();
    drop(native);
    drop(p);
    assert_eq!(
        super::observation::canonical(&retained.snapshot().unwrap()).unwrap(),
        stable,
        "owned result survives machine/provider drop"
    );
    let malformed = malformed(lua, module, &class, &parse, &build, parser, snapshot);
    let local_families = local::run(lua, module, &class, &parse, &build, parser, snapshot);
    assert_eq!(class.raw_get::<Function>("ParseRaw").unwrap(), parse);
    assert_eq!(class.raw_get::<Function>("BuildModList").unwrap(), build);
    json!({"variants":variants,"reuse":reuse,"malformed_finite_input":malformed,"local_families":local_families,
        "scope":{"declared_assembly_field_row_field_graph_contract":true,"original_saved_build_mutated":false,"source_outputs_injected_as_native_results":false,"actual_dependency_arity_observed":false,"dependency_order_parity":false,"owned_result_survives_machine_provider_drop":true}})
}
fn malformed(
    lua: &Lua,
    module: &Table,
    class: &Table,
    parse: &Function,
    build: &Function,
    parser: &Function,
    snapshot: &GameDataSnapshot,
) -> Json {
    let source = item(lua);
    let mut native = machine(snapshot);
    let mut p = provider(snapshot, parser);
    let raw = "Rarity: Normal\nGold Ring\nImplicits: 0\n+10 to maximum Life";
    let (_, event) = parse_step(
        lua,
        module,
        &source,
        parse,
        &mut native,
        &mut p,
        raw,
        "malformed specimen preparation",
    );
    let mut request = p.requests.first().unwrap().clone();
    assert!(request.previous.is_none());
    assert!(request.reparsed);
    // Original entry snapshot and original machine input are altered in the same
    // way. Neither observed source output nor a fabricated success enters native.
    let before: Table = event
        .raw_get::<Table>("before")
        .unwrap()
        .raw_get("root")
        .unwrap();
    before.set_metatable(Some(class.clone())).unwrap();
    before
        .raw_get::<Table>("requirements")
        .unwrap()
        .raw_set("str", Value::Nil)
        .unwrap();
    assert!(request.state.requirements.remove("str").is_some());
    let (source_error, report) = observe(lua, module, || build.call::<MultiValue>(before.clone()));
    let source_error = source_error.unwrap_err().to_string();
    assert!(
        source_error.contains("arithmetic")
            && source_error
                .replace('\\', "/")
                .contains("Classes/Item.lua:2846:"),
        "{source_error}"
    );
    let definitions = snapshot
        .item_assembly()
        .bind(
            snapshot.item_loading(),
            snapshot.item_scalability(),
            &snapshot.package().actor,
            snapshot.modifier_parser(),
        )
        .unwrap();
    let attempt = assembly::assemble(definitions, &request, &mut p.inner, None);
    let native_error = attempt.result.unwrap_err();
    assert_eq!(native_error.kind, assembly::AssemblyErrorKind::Source);
    let partial = attempt
        .partial
        .expect("source failure retains native mutation prefix");
    assert!(!partial.is_complete());
    let event = last(&report);
    assert!(!event.raw_get::<bool>("completed").unwrap());
    let compared = compare_graph(
        &partial,
        &event,
        "malformed missing requirement after collection prefix",
    );
    json!({"scope":"derived finite method input, not an original parser outcome","input_change":"remove requirements.str from same authentic entry inputs","native_stage":attempt.stage,"native_error":native_error.message,"source_error":source_error,"prefix":compared,"source_after_scope":event.raw_get::<String>("after_scope").unwrap(),"registration_authorized":false})
}
