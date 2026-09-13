//! Jewel cases invoke full original methods. Derived modifiers are explicit
//! finite pre-call inputs; original assembly outputs are never dependencies.
use super::{compare_graph, finish_step, item, last, machine, observe, parse_step, provider};
use mlua::{Function, Lua, MultiValue, Table, Value};
use poe_optimizer_data::{
    game_data::GameDataSnapshot,
    item_loading::{ItemMetadataTable as Metadata, ItemMetadataValue as Field},
};
use poe_optimizer_import::item_loading::assembly;
use serde_json::{Value as Json, json};
use std::collections::BTreeMap;

fn record(fields: impl IntoIterator<Item = (&'static str, Field)>) -> Metadata {
    Metadata {
        fields: fields.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        indexed: BTreeMap::new(),
    }
}
fn modifier(name: &str, kind: &str, value: Field) -> Metadata {
    record([
        ("name", Field::Text(name.into())),
        ("type", Field::Text(kind.into())),
        ("value", value),
        ("flags", Field::Number(0.0)),
        ("keywordFlags", Field::Number(0.0)),
    ])
}
fn override_value(name: &str, key: Option<&str>, value: Option<Field>) -> Metadata {
    let mut payload = Metadata::default();
    if let Some(key) = key {
        payload.fields.insert("key".into(), Field::Text(key.into()));
    }
    if let Some(value) = value {
        payload.fields.insert("value".into(), value);
    }
    modifier(name, "LIST", Field::Table(payload))
}
fn value(lua: &Lua, field: &Field, depth: usize) -> Value {
    assert!(depth <= 16, "directed fixture metadata depth");
    match field {
        Field::Boolean(v) => Value::Boolean(*v),
        Field::Number(v) => {
            assert!(v.is_finite());
            Value::Number(*v)
        }
        Field::Text(v) => {
            assert!(v.len() <= 4096);
            Value::String(lua.create_string(v).unwrap())
        }
        Field::Array(values) => {
            assert!(values.len() <= 128);
            Value::Table(
                lua.create_sequence_from(values.iter().map(|v| value(lua, v, depth + 1)))
                    .unwrap(),
            )
        }
        Field::Table(table) => Value::Table(table_value(lua, table, depth + 1)),
        Field::Callback(_) => panic!("directed fixture cannot erase callback input"),
    }
}
fn table_value(lua: &Lua, table: &Metadata, depth: usize) -> Table {
    assert!(depth <= 16 && table.fields.len() + table.indexed.len() <= 128);
    let out = lua.create_table().unwrap();
    for (k, v) in &table.fields {
        assert!(k.len() <= 4096);
        out.raw_set(k.as_str(), value(lua, v, depth + 1)).unwrap();
    }
    for (k, v) in &table.indexed {
        out.raw_set(*k, value(lua, v, depth + 1)).unwrap();
    }
    out
}
fn pack(lua: &Lua, modifiers: &[Metadata]) -> Table {
    assert!(modifiers.len() <= 128);
    lua.create_sequence_from(modifiers.iter().map(|m| table_value(lua, m, 0)))
        .unwrap()
}
fn graph(value: Table) -> Json {
    let graph = super::super::observation::capture(&[Value::Table(value)]).unwrap();
    super::super::observation::canonical(&graph).unwrap()
}

fn raw(base: &str) -> String {
    format!("Rarity: Normal\n{base}\nImplicits: 0\n+10 to maximum Life")
}
#[derive(Clone, Copy)]
enum Expected {
    Complete,
    SourceError,
}
struct Context<'a> {
    lua: &'a Lua,
    module: &'a Table,
    class: &'a Table,
    parse: &'a Function,
    build: &'a Function,
    parser: &'a Function,
    snapshot: &'a GameDataSnapshot,
}
impl Context<'_> {
    fn prepared(
        &self,
        label: &str,
    ) -> (
        Table,
        poe_optimizer_import::item_loading::AssemblyRequest,
        super::dependencies::Recorded<
            poe_optimizer_import::item_loading::NativeItemLoadProvider<
                '_,
                super::dependencies::OriginalParser,
            >,
        >,
    ) {
        let source = item(self.lua);
        let mut native = machine(self.snapshot);
        let mut dependency = provider(self.snapshot, self.parser);
        let (_, event) = parse_step(
            self.lua,
            self.module,
            &source,
            self.parse,
            &mut native,
            &mut dependency,
            &raw("Ruby"),
            label,
        );
        let request = dependency.requests.first().unwrap().clone();
        assert!(request.reparsed && request.previous.is_none());
        assert_eq!(request.state.explicit_mod_lines.len(), 1);
        let before: Table = event
            .raw_get::<Table>("before")
            .unwrap()
            .raw_get("root")
            .unwrap();
        before.set_metatable(Some(self.class.clone())).unwrap();
        (before, request, dependency)
    }
    fn derived(
        &self,
        label: &str,
        name: Option<&str>,
        modifiers: Vec<Metadata>,
        cluster: Option<Metadata>,
        expected: Expected,
    ) -> Json {
        let (before, mut request, mut dependency) = self.prepared(&format!("{label} entry"));
        let row: Table = before
            .raw_get::<Table>("explicitModLines")
            .unwrap()
            .raw_get(1)
            .unwrap();
        assert!(matches!(row.raw_get::<Value>("extra").unwrap(), Value::Nil));
        assert!(request.state.explicit_mod_lines[0].extra.is_none());
        row.raw_set("modList", pack(self.lua, &modifiers)).unwrap();
        request.state.explicit_mod_lines[0].modifiers = modifiers.clone();
        if let Some(name) = name {
            before.raw_set("name", name).unwrap();
            request.state.name = name.into();
        }
        if let Some(cluster) = &cluster {
            before
                .raw_set("clusterJewel", table_value(self.lua, cluster, 0))
                .unwrap();
        }
        request.state.cluster_jewel = cluster.clone();
        let input = graph(row.raw_get("modList").unwrap());
        assert_eq!(
            input,
            graph(pack(
                self.lua,
                &request.state.explicit_mod_lines[0].modifiers
            ))
        );
        if let Some(cluster) = &request.state.cluster_jewel {
            assert_eq!(
                graph(before.raw_get("clusterJewel").unwrap()),
                graph(table_value(self.lua, cluster, 0))
            );
        }
        let (source_result, report) = observe(self.lua, self.module, || {
            self.build.call::<MultiValue>(before.clone())
        });
        let definitions = self
            .snapshot
            .item_assembly()
            .bind(
                self.snapshot.item_loading(),
                self.snapshot.item_scalability(),
                &self.snapshot.package().actor,
                self.snapshot.modifier_parser(),
            )
            .unwrap();
        let attempt = assembly::assemble(definitions, &request, &mut dependency.inner, None);
        let event = last(&report);
        let outcome = match expected {
            Expected::Complete => {
                source_result.unwrap();
                assert!(event.raw_get::<bool>("completed").unwrap());
                let result = attempt.result.unwrap();
                assert!(result.is_complete());
                let data: Table = before.raw_get("jewelData").unwrap();
                // The List guard returns a table even when empty, so absence is not equivalent.
                assert!(matches!(
                    data.raw_get::<Value>("fromNothingKeystones").unwrap(),
                    Value::Table(_)
                ));
                if name.is_some_and(|name| name.contains("Grand Spectrum")) {
                    spectrum_alias(&before);
                }
                json!({"status":"complete","graph":compare_graph(&result,&event,label)})
            }
            Expected::SourceError => {
                let source_error = source_result.unwrap_err().to_string();
                assert!(!event.raw_get::<bool>("completed").unwrap());
                let native_error = attempt.result.unwrap_err();
                assert_eq!(
                    native_error.kind,
                    assembly::AssemblyErrorKind::Source,
                    "{label}"
                );
                let prefix = attempt.partial.expect("source error prefix");
                assert!(!prefix.is_complete());
                json!({"status":"source_error","source_error":source_error,"native_error":native_error.message,
                    "stage":attempt.stage,"prefix":compare_graph(&prefix,&event,label)})
            }
        };
        json!({"label":label,"scope":"derived identical finite pre-call inputs, not a parser-return or selected-tree-initialization claim",
            "name_override":name,"input_modifiers":modifiers,"input_cluster":cluster,
            "ordered_input_graph_sha256":super::super::hash(&serde_json::to_vec(&input).unwrap()),
            "same_ordered_native_source_inputs_verified":true,"source_poststate_used_as_dependency":false,
            "unreachable_slot_local_graph_observed":false,"outcome":outcome})
    }
    fn callback_frontier(&self) -> Json {
        use poe_optimizer_data::item_loading::{ItemOpaqueFunction, ItemSourceSpan};
        let (before, mut request, mut dependency) =
            self.prepared("actual callback ingress preparation");
        // Retain an actual original Lua Function as the value; do not execute or substitute it.
        let callback = self.parse.clone();
        let info = callback.info();
        let raw_path = info
            .source
            .as_deref()
            .unwrap()
            .strip_prefix('@')
            .unwrap()
            .replace('\\', "/");
        let suffix = raw_path.strip_prefix("src/").unwrap_or(&raw_path);
        let matches: Vec<_> = self
            .snapshot
            .item_loading()
            .data()
            .source
            .files
            .iter()
            .filter(|(path, _)| path.ends_with(suffix))
            .collect();
        assert_eq!(matches.len(), 1, "exact original callback file binding");
        let (path, sha) = matches[0];
        let descriptor = ItemOpaqueFunction {
            callback: ItemSourceSpan {
                path: path.clone(),
                line: info.line_defined.unwrap() as u32,
                end_line: info.last_line_defined.unwrap() as u32,
                sha256: sha.clone(),
            },
        };
        let source_mod = table_value(
            self.lua,
            &modifier("JewelFunc", "LIST", Field::Number(1.0)),
            0,
        );
        source_mod.raw_set("value", callback.clone()).unwrap();
        before
            .raw_get::<Table>("explicitModLines")
            .unwrap()
            .raw_get::<Table>(1)
            .unwrap()
            .raw_set(
                "modList",
                self.lua.create_sequence_from([source_mod]).unwrap(),
            )
            .unwrap();
        request.state.explicit_mod_lines[0].modifiers = vec![modifier(
            "JewelFunc",
            "LIST",
            Field::Callback(descriptor.clone()),
        )];
        let (result, report) = observe(self.lua, self.module, || {
            self.build.call::<MultiValue>(before.clone())
        });
        result.unwrap();
        assert_eq!(
            before
                .raw_get::<Table>("jewelData")
                .unwrap()
                .raw_get::<Table>("funcList")
                .unwrap()
                .raw_get::<Function>(1)
                .unwrap(),
            callback
        );
        assert!(last(&report).raw_get::<bool>("completed").unwrap());
        let definitions = self
            .snapshot
            .item_assembly()
            .bind(
                self.snapshot.item_loading(),
                self.snapshot.item_scalability(),
                &self.snapshot.package().actor,
                self.snapshot.modifier_parser(),
            )
            .unwrap();
        let attempt = assembly::assemble(definitions, &request, &mut dependency.inner, None);
        let error = attempt.result.unwrap_err();
        assert_eq!(error.kind, assembly::AssemblyErrorKind::Unsupported);
        assert!(attempt.partial.as_ref().is_none_or(|p| !p.is_complete()));
        json!({"scope":"actual original callback retained in source; source-bound opaque descriptor refused at native finite ingress",
            "descriptor":descriptor,"original_function_identity_preserved":true,"source_completed":true,
            "native_error":error.message,"native_stage":attempt.stage,"same_graph_parity_claimed":false,
            "source_error_prefix_parity_claimed":false,"native_callback_execution":false})
    }
}

// The existing assembly hook snapshots BuildModList return, which precedes
// ParseRaw's deferred/override writes. Capture the actual full-method return
// separately; never substitute an assembly-return snapshot for that boundary.
fn parse_return(lua: &Lua, module: &Table, source: &Table, parse: &Function, raw: &str) -> Table {
    let fields = lua
        .create_sequence_from(super::FIELDS.iter().copied())
        .unwrap();
    let capture: Table = module
        .raw_get::<Function>("start")
        .unwrap()
        .call(fields)
        .unwrap();
    struct Cleanup(Option<Function>);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            if let Some(f) = self.0.take() {
                let _ = f.call::<()>(());
            }
        }
    }
    let mut cleanup = Cleanup(Some(capture.raw_get::<Function>("finish").unwrap()));
    let result = parse.call::<MultiValue>((source.clone(), raw));
    let _: Table = cleanup.0.take().unwrap().call(()).unwrap();
    result.unwrap();
    let after: Table = capture
        .raw_get::<Function>("snapshot")
        .unwrap()
        .call(source.clone())
        .unwrap();
    let event = lua.create_table().unwrap();
    event.raw_set("after", after).unwrap();
    event.raw_set("completed", true).unwrap();
    event
        .raw_set("after_scope", "actual_ParseRaw_return")
        .unwrap();
    event
}
fn radius_histories(context: &Context<'_>) -> Json {
    use poe_optimizer_import::item_loading::{
        ItemLoadStatus, JewelRadiusContext, JewelRadiusProvenance,
    };
    let lua = context.lua;
    let data: Table = lua.globals().raw_get("data").unwrap();
    let setter: Function = data.raw_get("setJewelRadiiGlobally").unwrap();
    let versions: Table = lua.globals().raw_get("treeVersionList").unwrap();
    assert!(versions.raw_len() > 0 && versions.raw_len() <= 128);
    let first: String = versions.raw_get(1).unwrap();
    let latest: String = lua.globals().raw_get("latestTreeVersion").unwrap();
    assert_eq!(
        versions.raw_get::<String>(versions.raw_len()).unwrap(),
        latest
    );
    let source = item(lua);
    let mut native = machine(context.snapshot);
    let mut dependency = provider(context.snapshot, context.parser);
    let mut cases = Vec::new();
    let vectors = [
        (
            "static Small",
            "Ruby",
            "Radius: Small",
            "+10 to maximum Life",
        ),
        (
            "unmatched label retains prior index",
            "Ruby",
            "Radius: Unmatched Fixture Label",
            "+10 to maximum Life",
        ),
        (
            "absent label capture retains prior index",
            "Ruby",
            "Radius: 123",
            "+10 to maximum Life",
        ),
        (
            "Variable defers to parsed medium ring",
            "Ruby",
            "Radius: Variable",
            "Affects Passives in Medium Ring",
        ),
        (
            "Variable missing index removes prior scalar",
            "Ruby",
            "Radius: Variable",
            "+10 to maximum Life",
        ),
        (
            "Variable first word preserves complete label",
            "Ruby",
            "Radius: Variable Extra",
            "Affects Passives in Small Ring",
        ),
        (
            "static label then time-lost override",
            "Time-Lost Ruby",
            "Radius: Small",
            "Upgrades Radius to Large",
        ),
        (
            "deferred index then time-lost override",
            "Time-Lost Ruby",
            "Radius: Variable",
            "Affects Passives in Small Ring\nUpgrades Radius to Very Large",
        ),
        (
            "earlier Variable remains deferred after later static header",
            "Ruby",
            "Radius: Variable\nRadius: Small",
            "Affects Passives in Medium Ring",
        ),
        (
            "missing header retains prior radius fields",
            "Ruby",
            "",
            "+10 to maximum Life",
        ),
        (
            "nonjewel reparse preserves prior local graph",
            "Gold Ring",
            "",
            "+10 to maximum Life",
        ),
        (
            "jewel return resets local data and retains header fields",
            "Sapphire",
            "",
            "+10 to maximum Life",
        ),
    ];
    for (index, (label, base, headers, mods)) in vectors.iter().enumerate() {
        let requested = if index % 2 == 0 {
            first.as_str()
        } else {
            latest.as_str()
        };
        // An explicit original call establishes this specimen's context. No
        // final selected spec is substituted for an earlier import environment.
        setter.call::<()>(requested).unwrap();
        let radius = JewelRadiusContext::resolve(
            context.snapshot.item_loading(),
            requested,
            JewelRadiusProvenance::ExplicitCaller,
        )
        .unwrap();
        let current: Table = data.raw_get("jewelRadius").unwrap();
        let original_definitions = graph(current.clone());
        assert_eq!(
            original_definitions,
            graph(table_value(lua, radius.radii(), 0)),
            "resolved definitions must equal actual setter output"
        );
        assert_eq!(
            data.raw_get::<f64>("maxJewelRadius").unwrap().to_bits(),
            radius.evidence().maximum_radius.to_bits()
        );
        native.set_jewel_radius_context(radius.clone()).unwrap();
        let raw = format!("Rarity: Normal\n{base}\n{headers}\nImplicits: 0\n{mods}");
        let event = parse_return(lua, context.module, &source, context.parse, &raw);
        native.apply_text(&raw, &mut dependency).unwrap();
        assert_eq!(
            native.status(),
            ItemLoadStatus::Complete,
            "{label}: {:?}",
            native.pending()
        );
        let parsed = compare_graph(native.assembly_progress().unwrap(), &event, label);
        assert_eq!(data.raw_get::<Table>("jewelRadius").unwrap(), current);
        assert_eq!(
            graph(current),
            original_definitions,
            "item parse may not silently replace context definitions"
        );
        let final_graph = finish_step(
            lua,
            context.module,
            &source,
            context.build,
            &mut native,
            &mut dependency,
            &format!("{label} final assembly"),
        );
        cases.push(json!({"label":label,"raw":raw,"context":radius.evidence(),"actual_original_setter_called":true,"resolved_complete_radius_graph_sha256":super::super::hash(&serde_json::to_vec(&original_definitions).unwrap()),"parse_return":parsed,"final_assembly":final_graph}));
    }
    // Full original ParseRaw continues after BuildModList's NoBase return.
    // The Small header writes 1, then the retained time-lost override restores 4.
    setter.call::<()>(latest.as_str()).unwrap();
    let radius = JewelRadiusContext::resolve(
        context.snapshot.item_loading(),
        &latest,
        JewelRadiusProvenance::ExplicitCaller,
    )
    .unwrap();
    assert_eq!(
        graph(data.raw_get("jewelRadius").unwrap()),
        graph(table_value(lua, radius.radii(), 0))
    );
    let source_no_base = item(lua);
    let mut native_no_base = machine(context.snapshot);
    let mut no_base_provider = provider(context.snapshot, context.parser);
    native_no_base
        .set_jewel_radius_context(radius.clone())
        .unwrap();
    let initial = "Rarity: Normal\nTime-Lost Ruby\nRadius: Variable\nImplicits: 0\nAffects Passives in Small Ring\nUpgrades Radius to Very Large";
    let event = parse_return(lua, context.module, &source_no_base, context.parse, initial);
    native_no_base
        .apply_text(initial, &mut no_base_provider)
        .unwrap();
    assert_eq!(native_no_base.status(), ItemLoadStatus::Complete);
    let initial_graph = compare_graph(
        native_no_base.assembly_progress().unwrap(),
        &event,
        "NoBase radius initial Time-Lost item",
    );
    let retained: Table = source_no_base.raw_get("jewelData").unwrap();
    assert_eq!(
        source_no_base.raw_get::<f64>("jewelRadiusIndex").unwrap(),
        4.0
    );
    let missing = "Rarity: Normal\nNo matching R2ab base\nRadius: Small";
    let event = parse_return(lua, context.module, &source_no_base, context.parse, missing);
    native_no_base
        .apply_text(missing, &mut no_base_provider)
        .unwrap();
    assert!(matches!(
        source_no_base.raw_get::<Value>("base").unwrap(),
        Value::Nil
    ));
    assert_eq!(native_no_base.status(), ItemLoadStatus::NoBase);
    assert!(native_no_base.assembled().is_none());
    assert_eq!(
        source_no_base.raw_get::<Table>("jewelData").unwrap(),
        retained
    );
    assert_eq!(
        source_no_base
            .raw_get::<String>("jewelRadiusLabel")
            .unwrap(),
        "Small"
    );
    assert_eq!(
        source_no_base.raw_get::<f64>("jewelRadiusIndex").unwrap(),
        4.0,
        "retained override follows new Small header even without a base"
    );
    let prefix = native_no_base
        .assembly_progress()
        .expect("NoBase continuation must retain owned graph");
    assert!(!prefix.is_complete());
    let no_base_graph = compare_graph(
        prefix,
        &event,
        "NoBase radius header then retained override",
    );
    let ring = raw("Gold Ring");
    let event = parse_return(lua, context.module, &source_no_base, context.parse, &ring);
    native_no_base
        .apply_text(&ring, &mut no_base_provider)
        .unwrap();
    assert_eq!(native_no_base.status(), ItemLoadStatus::Complete);
    assert_eq!(
        source_no_base.raw_get::<Table>("jewelData").unwrap(),
        retained
    );
    assert_eq!(
        source_no_base.raw_get::<f64>("jewelRadiusIndex").unwrap(),
        4.0
    );
    let ring_graph = compare_graph(
        native_no_base.assembly_progress().unwrap(),
        &event,
        "valid accessory after NoBase radius tail",
    );
    let ring_final = finish_step(
        lua,
        context.module,
        &source_no_base,
        context.build,
        &mut native_no_base,
        &mut no_base_provider,
        "valid accessory final after NoBase radius tail",
    );
    assert_eq!(
        source_no_base.raw_get::<Table>("jewelData").unwrap(),
        retained
    );
    let no_base_history = json!({"initial_raw":initial,"missing_raw":missing,"following_raw":ring,"context":radius.evidence(),"initial":initial_graph,
        "no_base":{"status":"NoBase","registered":false,"complete_assembly_claimed":false,"graph":no_base_graph,"retained_source_jewel_table_identity":true,"header_index_overridden_after_early_assembly_return":true},"following_parse":ring_graph,"following_final":ring_final});
    // A caller which omits the required context remains an explicit dependency.
    let without = item(lua);
    let mut unbound = machine(context.snapshot);
    let mut p = provider(context.snapshot, context.parser);
    let raw = "Rarity: Normal\nRuby\nRadius: Small\nImplicits: 0\n+10 to maximum Life";
    let _ = parse_return(lua, context.module, &without, context.parse, raw);
    unbound.apply_text(raw, &mut p).unwrap();
    assert_eq!(unbound.status(), ItemLoadStatus::Pending);
    assert!(unbound.assembled().is_none());
    assert_eq!(
        data.raw_get::<Function>("setJewelRadiiGlobally").unwrap(),
        setter
    );
    json!({"cases":cases,"no_base_history":no_base_history,"missing_context":{"status":"explicit_dependency","pending":unbound.pending(),"source_completed":true,"native_graph_parity_claimed":false},
        "scope":"explicit original setter calls and matching caller-owned native contexts after import; not initial Build lifecycle evidence","initial_import_context_inferred_from_final_spec":false})
}
fn spectrum_alias(item: &Table) {
    let mods: Table = item.raw_get("modList").unwrap();
    let mut spectrum = None;
    let mut nested = None;
    for index in 1..=mods.raw_len() {
        let row: Table = mods.raw_get(index).unwrap();
        match row.raw_get::<String>("name").unwrap().as_str() {
            "Multiplier:GrandSpectrum" => {
                assert!(spectrum.is_none());
                spectrum = Some(row)
            }
            "MinionModifier" => {
                let payload: Table = row.raw_get("value").unwrap();
                if let Value::Table(value) = payload.raw_get::<Value>("mod").unwrap() {
                    nested = Some(value);
                }
            }
            _ => {}
        }
    }
    assert_eq!(
        spectrum.expect("actual spectrum row"),
        nested.expect("actual spectrum nested alias")
    );
}
fn cluster(min: f64, max: f64) -> Metadata {
    record([
        ("minNodes", Field::Number(min)),
        ("maxNodes", Field::Number(max)),
        (
            "skills",
            Field::Table(record([
                ("affliction_curse_effect", Field::Boolean(true)),
                ("affliction_curse_effect_small", Field::Boolean(true)),
                ("plain", Field::Boolean(true)),
            ])),
        ),
    ])
}
fn jewel(key: &str, value: Field) -> Metadata {
    override_value("JewelData", Some(key), Some(value))
}
#[allow(clippy::too_many_arguments)]
pub(super) fn run(
    lua: &Lua,
    module: &Table,
    class: &Table,
    parse: &Function,
    build: &Function,
    parser: &Function,
    snapshot: &GameDataSnapshot,
) -> Json {
    let context = Context {
        lua,
        module,
        class,
        parse,
        build,
        parser,
        snapshot,
    };
    let mut fresh = Vec::new();
    for base in ["Ruby", "Emerald", "Sapphire", "Diamond"] {
        let source = item(lua);
        let mut native = machine(snapshot);
        let mut p = provider(snapshot, parser);
        let (parsed, _) = parse_step(
            lua,
            module,
            &source,
            parse,
            &mut native,
            &mut p,
            &raw(base),
            &format!("plain {base}"),
        );
        let finished = finish_step(
            lua,
            module,
            &source,
            build,
            &mut native,
            &mut p,
            &format!("plain {base} final"),
        );
        fresh.push(json!({"base":base,"parse":parsed,"final":finished}));
    }
    let source = item(lua);
    let mut native = machine(snapshot);
    let mut p = provider(snapshot, parser);
    let mut histories = Vec::new();
    let mut prior = None;
    for base in [
        "Ruby",
        "Gold Ring",
        "Sapphire",
        "Painted Tower Shield",
        "Emerald",
    ] {
        let (parsed, _) = parse_step(
            lua,
            module,
            &source,
            parse,
            &mut native,
            &mut p,
            &raw(base),
            &format!("jewel cross-family {base}"),
        );
        let current = source.raw_get::<Value>("jewelData").unwrap();
        // Every BuildModList resets jewelData only for the Jewel branch; other branches retain it.
        if let Value::Table(current) = current
            && let Some(old) = &prior
        {
            if ["Ruby", "Sapphire", "Emerald"].contains(&base) {
                assert_ne!(old, &current);
            } else {
                assert_eq!(old, &current);
            }
        }
        let finished = finish_step(
            lua,
            module,
            &source,
            build,
            &mut native,
            &mut p,
            &format!("jewel cross-family {base} final"),
        );
        prior = match source.raw_get::<Value>("jewelData").unwrap() {
            Value::Table(t) => Some(t),
            _ => None,
        };
        histories.push(json!({"base":base,"parse":parsed,"final":finished}));
    }
    let complete = Expected::Complete;
    let error = Expected::SourceError;
    let mut cases = vec![
        context.derived(
            "Grand Spectrum shared minion modifier",
            Some("Directed Grand Spectrum Ruby"),
            vec![],
            None,
            complete,
        ),
        context.derived(
            "finite ordered function-list values",
            None,
            vec![
                modifier(
                    "JewelFunc",
                    "LIST",
                    Field::Text("retained noncallable".into()),
                ),
                modifier(
                    "JewelFunc",
                    "LIST",
                    Field::Table(record([("payload", Field::Number(7.0))])),
                ),
                modifier("JewelFunc", "LIST", Field::Number(0.0)),
            ],
            None,
            complete,
        ),
        context.derived(
            "ordered overrides and last alternate class",
            None,
            vec![
                jewel(
                    "custom",
                    Field::Table(record([(
                        "nested",
                        Field::Array(vec![Field::Boolean(false), Field::Number(2.5)]),
                    )])),
                ),
                jewel("radiusIndex", Field::Number(3.0)),
                override_value("JewelData", Some("radiusIndex"), None),
                jewel("funcList", Field::Boolean(false)),
                modifier("AlternateClassStart", "LIST", Field::Text("first".into())),
                modifier("AlternateClassStart", "LIST", Field::Number(0.0)),
            ],
            None,
            complete,
        ),
        context.derived(
            "FromNothing key order replacement and nil",
            None,
            vec![
                override_value(
                    "FromNothingKeystones",
                    Some("one"),
                    Some(Field::Number(1.0)),
                ),
                override_value(
                    "FromNothingKeystones",
                    Some("two"),
                    Some(Field::Table(record([("alias", Field::Boolean(true))]))),
                ),
                override_value("FromNothingKeystones", Some("one"), None),
            ],
            None,
            complete,
        ),
        context.derived(
            "JewelData nil key after prefix",
            None,
            vec![
                jewel("prefix", Field::Number(9.0)),
                override_value("JewelData", None, Some(Field::Number(1.0))),
            ],
            None,
            error,
        ),
        context.derived(
            "FromNothing nil key after jewel writes",
            None,
            vec![
                jewel("prefix", Field::Number(9.0)),
                override_value("FromNothingKeystones", None, Some(Field::Number(1.0))),
            ],
            None,
            error,
        ),
        context.derived(
            "JewelData truthy scalar index failure",
            None,
            vec![modifier("JewelData", "LIST", Field::Number(7.0))],
            None,
            error,
        ),
    ];
    for (label, skill, count) in [
        (
            "small curse correction before clamp",
            "affliction_curse_effect",
            3.0,
        ),
        ("medium curse no correction", "affliction_curse_effect", 4.0),
        ("lower clamp", "plain", 0.0),
        ("upper clamp", "plain", 99.0),
        ("invalid skill removed", "absent", 5.0),
    ] {
        cases.push(context.derived(
            label,
            None,
            vec![
                jewel("clusterJewelSkill", Field::Text(skill.into())),
                jewel("clusterJewelNodeCount", Field::Number(count)),
                modifier("ClusterJewelNotable", "LIST", Field::Text("one".into())),
                modifier("ClusterJewelNotable", "LIST", Field::Text("two".into())),
                modifier(
                    "AddToClusterJewelNode",
                    "LIST",
                    Field::Text("+10 to maximum Life".into()),
                ),
            ],
            Some(cluster(2.0, 8.0)),
            complete,
        ));
    }
    for (label, mods) in [
        (
            "value-valued keystone validity",
            vec![jewel(
                "clusterJewelKeystone",
                Field::Table(record([("keystone", Field::Number(1.0))])),
            )],
        ),
        (
            "zero truthy nothingness validity",
            vec![
                jewel("clusterJewelSocketCountOverride", Field::Number(0.0)),
                jewel("clusterJewelNothingnessCount", Field::Number(0.0)),
            ],
        ),
        (
            "false cluster validity",
            vec![
                jewel("clusterJewelSmallsAreNothingness", Field::Boolean(false)),
                jewel("clusterJewelSocketCountOverride", Field::Boolean(false)),
            ],
        ),
    ] {
        cases.push(context.derived(label, None, mods, Some(cluster(2.0, 8.0)), complete));
    }
    cases.push(context.derived(
        "malformed cluster count after collected lists",
        None,
        vec![
            jewel(
                "clusterJewelSkill",
                Field::Text("affliction_curse_effect".into()),
            ),
            jewel("clusterJewelNodeCount", Field::Boolean(true)),
            modifier(
                "ClusterJewelNotable",
                "LIST",
                Field::Text("retained".into()),
            ),
        ],
        Some(cluster(2.0, 8.0)),
        error,
    ));
    let callbacks = context.callback_frontier();
    let radius = radius_histories(&context);
    json!({"fresh":fresh,"cross_family_reparse":histories,"derived":cases,"callback_ingress":callbacks,"radius_histories":radius,
        "scope":{"whole_declared_jewel_graph":true,"radius_headers":"explicit caller setter/context histories; initial import lifecycle remains separate","cluster_input":"derived caller metadata; original pin does not initialize clusterJewel","source_poststate_used_as_dependency":false,"whole_native_build":false}})
}
