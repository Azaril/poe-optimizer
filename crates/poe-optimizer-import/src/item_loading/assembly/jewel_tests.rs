use super::*;
use crate::item_loading::{
    AssemblyExecution, BuiltinItemLoadProvider, ItemLoadMachine, ItemLoadStatus,
    NativeItemLoadProvider, ParseOutcome, ParseRequest, UnavailableItemLoadProvider,
};
use poe_optimizer_data::{
    game_data::{GameDataSnapshot, bundled_snapshot},
    item_assembly::{ItemAssemblyCatalog, ItemAssemblyOverridePolicy},
    item_loading::{ItemMetadataTable, ItemMetadataValue, ItemOpaqueFunction, ItemSourceSpan},
};
use std::sync::OnceLock;

fn data() -> &'static GameDataSnapshot {
    static DATA: OnceLock<GameDataSnapshot> = OnceLock::new();
    DATA.get_or_init(|| bundled_snapshot().unwrap())
}
fn definitions() -> ItemAssemblyDefinitions<'static> {
    data()
        .item_assembly()
        .bind(
            data().item_loading(),
            data().item_scalability(),
            &data().package().actor,
            data().modifier_parser(),
        )
        .unwrap()
}
fn request() -> AssemblyRequest {
    #[derive(Default)]
    struct Capture(Option<AssemblyRequest>);
    impl ItemLoadProvider for Capture {
        fn parse_modifier(&mut self, _: &ParseRequest) -> DependencyResult<ParseOutcome> {
            DependencyResult::Available(ParseOutcome {
                modifiers: Some(Vec::new()),
                extra: None,
            })
        }
        fn assemble_with_trace(&mut self, request: &AssemblyRequest) -> AssemblyExecution {
            self.0 = Some(request.clone());
            AssemblyExecution {
                outcome: DependencyResult::Unavailable("capture input".into()),
                prefix: None,
            }
        }
    }
    let mut machine = ItemLoadMachine::new(data().item_loading());
    let mut capture = Capture::default();
    machine
        .apply_text("Rarity: Normal\nRuby\nImplicits: 0", &mut capture)
        .unwrap();
    capture.0.unwrap()
}
type C<'d, 'r> = Context<'d, 'r, UnavailableItemLoadProvider>;
fn setup<'d, 'r>(
    defs: ItemAssemblyDefinitions<'d>,
    request: &'r AssemblyRequest,
    dependencies: &'r mut UnavailableItemLoadProvider,
    limits: AssemblyLimits,
) -> (C<'d, 'r>, TableId, TableId) {
    let mut arena = Arena::new(limits);
    let root = arena.new_table().unwrap();
    let mut c = Context {
        arena,
        root,
        definitions: defs,
        request,
        dependencies,
        stage: "test",
        patterns: text::Patterns::new(limits),
        sequence: 0,
    };
    c.set(
        &defs.policy().jewel.grand_spectrum.name_item_field,
        Value::Text("finite jewel".into()),
    )
    .unwrap();
    let output = c.fresh_field(&defs.policy().jewel.output_field).unwrap();
    let list = c.mod_list().unwrap();
    (c, output, list)
}
fn add(c: &mut C<'_, '_>, list: TableId, query: &str, value: Value) -> TableId {
    let kind = &c.definitions.policy().kinds.list;
    c.add_new(list, query, kind, value, None).unwrap();
    let index = c.arena.dense_len(list).unwrap();
    table(c.arena.get_index(list, index as i64).unwrap()).unwrap()
}
fn entry(
    c: &mut C<'_, '_>,
    list: TableId,
    policy: &ItemAssemblyOverridePolicy,
    key: Value,
    value: Value,
) -> TableId {
    let payload = c.arena.new_table().unwrap();
    c.arena.set_field(payload, &policy.key_field, key).unwrap();
    c.arena
        .set_field(payload, &policy.value_field, value)
        .unwrap();
    add(c, list, &policy.query_name, Value::Table(payload))
}
fn cluster(c: &mut C<'_, '_>, minimum: Value, maximum: Value) -> (TableId, TableId) {
    let p = &c.definitions.policy().jewel.cluster;
    let cluster = c.fresh_field(&p.item_field).unwrap();
    c.arena
        .set_field(cluster, &p.min_nodes_field, minimum)
        .unwrap();
    c.arena
        .set_field(cluster, &p.max_nodes_field, maximum)
        .unwrap();
    let skills = c.arena.new_table().unwrap();
    c.arena
        .set_field(cluster, &p.skills_field, Value::Table(skills))
        .unwrap();
    (cluster, skills)
}
fn field(c: &mut C<'_, '_>, id: TableId, key: &str) -> Value {
    c.arena.get_field(id, key).unwrap()
}

#[test]
fn spectrum_modifier_is_shared_with_minion_payload_and_source_name() {
    let r = request();
    let mut d = UnavailableItemLoadProvider;
    let defs = definitions();
    let (mut c, output, list) = setup(defs, &r, &mut d, AssemblyLimits::default());
    let p = &defs.policy().jewel;
    let name = "Finite Grand Spectrum Jewel";
    c.set(&p.grand_spectrum.name_item_field, Value::Text(name.into()))
        .unwrap();
    c.local_jewel(list).unwrap();
    assert_eq!(c.arena.dense_len(list).unwrap(), 2);
    let spectrum = table(c.arena.get_index(list, 1).unwrap()).unwrap();
    let wrapper = table(c.arena.get_index(list, 2).unwrap()).unwrap();
    let payload = table(field(&mut c, wrapper, "value")).unwrap();
    assert_eq!(
        field(&mut c, payload, &p.grand_spectrum.nested_mod_field),
        Value::Table(spectrum)
    );
    assert_eq!(
        field(&mut c, spectrum, "name"),
        Value::Text(p.grand_spectrum.modifier_name.clone())
    );
    assert_eq!(
        field(&mut c, spectrum, "value"),
        Value::Number(p.grand_spectrum.modifier_value)
    );
    for record in [spectrum, wrapper] {
        assert_eq!(field(&mut c, record, "source"), Value::Text(name.into()));
    }
    c.arena
        .set_field(spectrum, "value", Value::Number(17.0))
        .unwrap();
    let shared = table(field(&mut c, payload, &p.grand_spectrum.nested_mod_field)).unwrap();
    assert_eq!(field(&mut c, shared, "value"), Value::Number(17.0));
    assert!(
        field(&mut c, output, &p.from_nothing.output_field)
            .as_table()
            .is_some()
    );
}

#[test]
fn ordered_lists_keep_finite_values_aliases_last_class_and_nil_stores() {
    let r = request();
    let mut d = UnavailableItemLoadProvider;
    let defs = definitions();
    let (mut c, output, list) = setup(defs, &r, &mut d, AssemblyLimits::default());
    let p = &defs.policy().jewel;
    let nested = c.arena.new_table().unwrap();
    for value in [
        Value::Number(0.0),
        Value::Boolean(false),
        Value::Table(nested),
    ] {
        add(&mut c, list, &p.functions.query_name, value);
    }
    let source = entry(
        &mut c,
        list,
        &p.overrides,
        Value::Text("custom".into()),
        Value::Table(nested),
    );
    add(
        &mut c,
        list,
        &p.alternate_class_start.query_name,
        Value::Text("first".into()),
    );
    add(
        &mut c,
        list,
        &p.alternate_class_start.query_name,
        Value::Table(nested),
    );
    entry(
        &mut c,
        list,
        &p.from_nothing.entries,
        Value::Text("removed".into()),
        Value::Number(1.0),
    );
    entry(
        &mut c,
        list,
        &p.from_nothing.entries,
        Value::Text("removed".into()),
        Value::Nil,
    );
    entry(
        &mut c,
        list,
        &p.from_nothing.entries,
        Value::Number(9.0),
        Value::Table(nested),
    );
    c.local_jewel(list).unwrap();
    let funcs = table(field(&mut c, output, &p.functions.output_field)).unwrap();
    assert_eq!(c.arena.dense_len(funcs).unwrap(), 2);
    assert_eq!(c.arena.get_index(funcs, 1).unwrap(), Value::Number(0.0));
    assert_eq!(c.arena.get_index(funcs, 2).unwrap(), Value::Table(nested));
    assert_eq!(field(&mut c, output, "custom"), Value::Table(nested));
    let payload = table(field(&mut c, source, "value")).unwrap();
    assert_eq!(
        field(&mut c, payload, &p.overrides.value_field),
        Value::Table(nested)
    );
    assert_eq!(
        field(&mut c, output, &p.alternate_class_start.output_field),
        Value::Table(nested)
    );
    let map = table(field(&mut c, output, &p.from_nothing.output_field)).unwrap();
    assert_eq!(field(&mut c, map, "removed"), Value::Nil);
    assert_eq!(c.arena.get_index(map, 9).unwrap(), Value::Table(nested));
}

#[test]
fn cluster_correction_lists_and_value_valued_validity_preserve_aliases() {
    let r = request();
    let mut d = UnavailableItemLoadProvider;
    let defs = definitions();
    let (mut c, output, list) = setup(defs, &r, &mut d, AssemblyLimits::default());
    let p = &defs.policy().jewel.cluster;
    let (_, skills) = cluster(&mut c, Value::Number(2.0), Value::Number(5.0));
    c.arena
        .set_field(skills, &p.correction.replacement_skill, Value::Number(0.0))
        .unwrap();
    c.arena
        .set_field(
            output,
            &p.skill_field,
            Value::Text(p.correction.matching_skill.clone()),
        )
        .unwrap();
    c.arena
        .set_field(output, &p.node_count_field, Value::Number(3.0))
        .unwrap();
    let nested = c.arena.new_table().unwrap();
    add(&mut c, list, &p.notables.query_name, Value::Table(nested));
    add(
        &mut c,
        list,
        &p.added_mods.query_name,
        Value::Text("finite node line".into()),
    );
    c.local_jewel(list).unwrap();
    assert_eq!(
        field(&mut c, output, &p.skill_field),
        Value::Text(p.correction.replacement_skill.clone())
    );
    assert_eq!(
        field(&mut c, output, &p.validity.output_field),
        Value::Number(3.0)
    );
    let notables = table(field(&mut c, output, &p.notables.output_field)).unwrap();
    assert_eq!(
        c.arena.get_index(notables, 1).unwrap(),
        Value::Table(nested)
    );
    c.arena
        .set_field(output, &p.validity.keystone_field, Value::Table(nested))
        .unwrap();
    c.local_jewel(list).unwrap();
    assert_eq!(
        field(&mut c, output, &p.validity.output_field),
        Value::Table(nested)
    );
    assert_ne!(
        field(&mut c, output, &p.notables.output_field),
        Value::Table(notables)
    );
}

#[test]
fn cluster_clamp_coerces_numeric_strings_but_preserves_zero_and_raw_skill_lookup() {
    let r = request();
    let mut d = UnavailableItemLoadProvider;
    let defs = definitions();
    let p = &defs.policy().jewel.cluster;
    for (count, minimum, maximum, expected) in [
        (
            Value::Text("99".into()),
            Value::Text("2".into()),
            Value::Text("5".into()),
            5.0f64,
        ),
        (
            Value::Number(0.0),
            Value::Number(-0.0),
            Value::Number(-0.0),
            -0.0f64,
        ),
    ] {
        let (mut c, output, list) = setup(defs, &r, &mut d, AssemblyLimits::default());
        let (_, skills) = cluster(&mut c, minimum, maximum);
        c.arena
            .set_index(skills, 7, Value::Text("truthy".into()))
            .unwrap();
        c.arena
            .set_field(output, &p.skill_field, Value::Number(7.0))
            .unwrap();
        c.arena
            .set_field(output, &p.node_count_field, count)
            .unwrap();
        c.local_jewel(list).unwrap();
        assert_eq!(
            field(&mut c, output, &p.node_count_field)
                .as_number()
                .unwrap()
                .to_bits(),
            expected.to_bits()
        );
        assert_eq!(
            field(&mut c, output, &p.validity.output_field)
                .as_number()
                .unwrap()
                .to_bits(),
            expected.to_bits()
        );
        // Boolean keys cannot occur in this represented text/integer map.
        c.arena
            .set_field(output, &p.skill_field, Value::Boolean(true))
            .unwrap();
        c.arena
            .set_field(
                output,
                &p.validity.socket_count_override_field,
                Value::Number(0.0),
            )
            .unwrap();
        c.arena
            .set_field(
                output,
                &p.validity.nothingness_count_field,
                Value::Text("selected value".into()),
            )
            .unwrap();
        c.local_jewel(list).unwrap();
        assert_eq!(field(&mut c, output, &p.skill_field), Value::Nil);
        assert_eq!(
            field(&mut c, output, &p.validity.output_field),
            Value::Text("selected value".into())
        );
    }
}

#[test]
fn malformed_store_and_strict_comparison_keep_reached_prefixes() {
    let r = request();
    let mut d = UnavailableItemLoadProvider;
    let defs = definitions();
    let p = &defs.policy().jewel;
    let (mut c, output, list) = setup(defs, &r, &mut d, AssemblyLimits::default());
    entry(
        &mut c,
        list,
        &p.overrides,
        Value::Text("before".into()),
        Value::Number(9.0),
    );
    add(
        &mut c,
        list,
        &p.alternate_class_start.query_name,
        Value::Text("last".into()),
    );
    let old = c.arena.new_table().unwrap();
    c.arena
        .set_field(output, &p.from_nothing.output_field, Value::Table(old))
        .unwrap();
    entry(
        &mut c,
        list,
        &p.from_nothing.entries,
        Value::Nil,
        Value::Number(1.0),
    );
    assert_eq!(
        c.local_jewel(list).unwrap_err().kind,
        AssemblyErrorKind::Source
    );
    assert_eq!(field(&mut c, output, "before"), Value::Number(9.0));
    assert_eq!(
        field(&mut c, output, &p.alternate_class_start.output_field),
        Value::Text("last".into())
    );
    let map = table(field(&mut c, output, &p.from_nothing.output_field)).unwrap();
    assert_ne!(map, old);
    assert!(c.arena.table(map).unwrap().fields.is_empty());

    let (mut c, output, list) = setup(defs, &r, &mut d, AssemblyLimits::default());
    cluster(&mut c, Value::Number(2.0), Value::Number(5.0));
    c.arena
        .set_field(
            output,
            &p.cluster.skill_field,
            Value::Text(p.cluster.correction.matching_skill.clone()),
        )
        .unwrap();
    c.arena
        .set_field(output, &p.cluster.node_count_field, Value::Text("3".into()))
        .unwrap();
    assert_eq!(
        c.local_jewel(list).unwrap_err().kind,
        AssemblyErrorKind::Source
    );
    assert_eq!(
        field(&mut c, output, &p.cluster.node_count_field),
        Value::Text("3".into())
    );
    for key in [
        &p.cluster.notables.output_field,
        &p.cluster.added_mods.output_field,
    ] {
        assert!(field(&mut c, output, key).as_table().is_some());
    }
    assert_eq!(
        field(&mut c, output, &p.cluster.validity.output_field),
        Value::Nil
    );
}

#[test]
fn empty_from_nothing_replaces_map_and_second_query_can_exhaust_budget() {
    let r = request();
    let mut d = UnavailableItemLoadProvider;
    let defs = definitions();
    let p = &defs.policy().jewel;
    let (mut c, output, list) = setup(defs, &r, &mut d, AssemblyLimits::default());
    let old = c.arena.new_table().unwrap();
    c.arena
        .set_field(output, &p.from_nothing.output_field, Value::Table(old))
        .unwrap();
    let before = c.arena.usage().tables;
    c.local_jewel(list).unwrap();
    assert_eq!(c.arena.usage().tables - before, 6);
    assert_ne!(
        field(&mut c, output, &p.from_nothing.output_field),
        Value::Table(old)
    );
    let first = field(&mut c, output, &p.from_nothing.output_field);
    c.local_jewel(list).unwrap();
    assert_ne!(field(&mut c, output, &p.from_nothing.output_field), first);

    let limits = AssemblyLimits {
        max_tables: 11,
        ..AssemblyLimits::default()
    };
    let (mut c, output, list) = setup(defs, &r, &mut d, limits);
    assert_eq!(c.arena.usage().tables, 6);
    // Five allocations admit the three earlier queries, guard query and new map;
    // the independently repeated entries query must still require its own table.
    assert_eq!(
        c.local_jewel(list).unwrap_err().kind,
        AssemblyErrorKind::Resource
    );
    let map = table(field(&mut c, output, &p.from_nothing.output_field)).unwrap();
    assert_eq!(c.arena.usage().tables, 11);
    assert!(c.arena.table(map).unwrap().fields.is_empty());
}

#[test]
fn changed_policy_fields_queries_pattern_and_threshold_are_consumed() {
    let mut raw = data().item_assembly().data().clone();
    let p = &mut raw.policy.jewel;
    p.output_field = "modelJewel".into();
    p.grand_spectrum.name_item_field = "modelName".into();
    p.grand_spectrum.name_pattern = "^Shared %d+$".into();
    p.grand_spectrum.modifier_name = "modelSpectrum".into();
    p.grand_spectrum.modifier_value = 2.0;
    p.grand_spectrum.minion_name = "modelMinion".into();
    p.grand_spectrum.nested_mod_field = "shared".into();
    p.functions.query_name = "modelFunctions".into();
    p.functions.output_field = "functions".into();
    p.from_nothing.guard_query_name = "emptyGuard".into();
    p.from_nothing.output_field = "modelMap".into();
    p.from_nothing.entries = ItemAssemblyOverridePolicy {
        query_name: "modelEntries".into(),
        key_field: "k".into(),
        value_field: "v".into(),
    };
    p.cluster.item_field = "modelCluster".into();
    p.cluster.skills_field = "allowed".into();
    p.cluster.min_nodes_field = "minimum".into();
    p.cluster.max_nodes_field = "maximum".into();
    p.cluster.skill_field = "skill".into();
    p.cluster.node_count_field = "nodes".into();
    p.cluster.correction.matching_skill = "old".into();
    p.cluster.correction.replacement_skill = "new".into();
    p.cluster.correction.node_count_below = 10.0;
    p.cluster.validity.output_field = "valid".into();
    let owner = ItemAssemblyCatalog::new(raw).unwrap();
    let defs = owner
        .bind(
            data().item_loading(),
            data().item_scalability(),
            &data().package().actor,
            data().modifier_parser(),
        )
        .unwrap();
    let r = request();
    let mut d = UnavailableItemLoadProvider;
    let (mut c, output, list) = setup(defs, &r, &mut d, AssemblyLimits::default());
    let p = &defs.policy().jewel;
    c.set("modelName", Value::Text("Shared 42".into())).unwrap();
    let (_, skills) = cluster(&mut c, Value::Number(1.0), Value::Number(8.0));
    c.arena
        .set_field(skills, "new", Value::Boolean(true))
        .unwrap();
    c.arena
        .set_field(output, "skill", Value::Text("old".into()))
        .unwrap();
    c.arena
        .set_field(output, "nodes", Value::Number(9.0))
        .unwrap();
    entry(
        &mut c,
        list,
        &p.from_nothing.entries,
        Value::Text("kept".into()),
        Value::Number(17.0),
    );
    add(&mut c, list, &p.functions.query_name, Value::Number(23.0));
    c.local_jewel(list).unwrap();
    assert_eq!(field(&mut c, output, "valid"), Value::Number(8.0));
    assert_eq!(field(&mut c, output, "skill"), Value::Text("new".into()));
    let map = table(field(&mut c, output, "modelMap")).unwrap();
    assert_eq!(field(&mut c, map, "kept"), Value::Number(17.0));
    let funcs = table(field(&mut c, output, "functions")).unwrap();
    assert_eq!(c.arena.get_index(funcs, 1).unwrap(), Value::Number(23.0));
    let modifier = table(c.arena.get_index(list, 3).unwrap()).unwrap();
    let wrapper = table(c.arena.get_index(list, 4).unwrap()).unwrap();
    assert_eq!(field(&mut c, modifier, "value"), Value::Number(2.0));
    let nested = table(field(&mut c, wrapper, "value")).unwrap();
    assert_eq!(field(&mut c, nested, "shared"), Value::Table(modifier));
    assert_eq!(c.get("jewelData").unwrap(), Value::Nil);
}

#[test]
fn production_provider_keeps_jewel_graph_and_resets_it_on_final_reassembly() {
    let mut provider = BuiltinItemLoadProvider::new(data());
    let mut machine = ItemLoadMachine::new(data().item_loading());
    machine.set_xml_attributes(&[("id".into(), "1".into())].into());
    machine
        .apply_text("Rarity: Normal\nRuby\nImplicits: 0", &mut provider)
        .unwrap();
    let first = machine.assembly_progress().unwrap().clone();
    let p = &definitions().policy().jewel;
    let old = first
        .field(first.root(), &p.output_field)
        .unwrap()
        .as_table()
        .unwrap();
    machine.finish_load(&mut provider).unwrap();
    let final_item = machine
        .assembled()
        .expect("ordinary jewel must complete through native provider");
    let current = final_item
        .field(final_item.root(), &p.output_field)
        .unwrap()
        .as_table()
        .unwrap();
    assert_ne!(old, current);
    let map = final_item
        .field(current, &p.from_nothing.output_field)
        .unwrap()
        .as_table()
        .unwrap();
    assert!(final_item.table(map).unwrap().fields.is_empty());
    assert!(
        final_item
            .field(final_item.root(), "modList")
            .unwrap()
            .as_table()
            .is_some()
    );
    assert!(first.field(first.root(), &p.output_field).is_some());
}

#[test]
fn opaque_jewel_function_is_explicitly_refused_without_a_complete_artifact() {
    struct Parser;
    impl ItemLoadProvider for Parser {
        fn parse_modifier(&mut self, _: &ParseRequest) -> DependencyResult<ParseOutcome> {
            let p = &definitions().policy().jewel.functions;
            let callback = ItemMetadataValue::Callback(ItemOpaqueFunction {
                callback: ItemSourceSpan {
                    path: "src/finite-jewel.lua".into(),
                    line: 1,
                    end_line: 2,
                    sha256: "0".repeat(64),
                },
            });
            DependencyResult::Available(ParseOutcome {
                modifiers: Some(vec![ItemMetadataTable {
                    fields: [
                        ("name".into(), ItemMetadataValue::Text(p.query_name.clone())),
                        (
                            "type".into(),
                            ItemMetadataValue::Text(definitions().policy().kinds.list.clone()),
                        ),
                        ("flags".into(), ItemMetadataValue::Number(0.0)),
                        ("keywordFlags".into(), ItemMetadataValue::Number(0.0)),
                        ("value".into(), callback),
                    ]
                    .into(),
                    indexed: Default::default(),
                }]),
                extra: None,
            })
        }
    }
    let mut provider = NativeItemLoadProvider::with_native_assembly(data(), Parser);
    let mut machine = ItemLoadMachine::new(data().item_loading());
    machine.set_xml_attributes(&[("id".into(), "1".into())].into());
    machine
        .apply_text(
            "Rarity: Normal\nRuby\nImplicits: 0\nfinite callback",
            &mut provider,
        )
        .unwrap();
    assert_eq!(machine.status(), ItemLoadStatus::Pending);
    assert!(
        machine
            .pending()
            .unwrap()
            .message
            .contains("opaque item callback")
    );
    assert!(machine.assembled().is_none());
    assert!(machine.assembly_progress().is_none());
}
