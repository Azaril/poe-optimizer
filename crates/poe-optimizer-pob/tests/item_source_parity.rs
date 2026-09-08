//! Original XML/ItemsTab load instructions, independent of item numerical parsing.
#![cfg(not(target_arch = "wasm32"))]
use mlua::{Function, HookTriggers, Lua, Table, Value, VmState};
use poe_optimizer_import::{
    item_source::{self, ItemSourceKind, ItemSourceNode, ItemSourceUse},
    source_xml::{PobContentEntry, PobTextKind, SourceContentKind},
};
use poe_optimizer_pob::source;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::OnceLock,
    time::{Duration, Instant},
};

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn original(path: &str) -> &'static str {
    static SOURCES: OnceLock<BTreeMap<&str, String>> = OnceLock::new();
    SOURCES.get_or_init(|| {
        [
            "runtime/lua/xml.lua",
            "src/Classes/ItemsTab.lua",
            "src/Classes/PassiveSpec.lua",
        ]
        .into_iter()
        .map(|path| {
            (
                path,
                source::read_verified_text(
                    &repository().join("vendor/path-of-building-poe2"),
                    path,
                )
                .unwrap(),
            )
        })
        .collect()
    })[path]
        .as_str()
}
fn section<'a>(source: &'a str, first: &str, next: &str) -> &'a str {
    assert_eq!(source.matches(first).count(), 1, "{first}");
    let start = source.find(first).unwrap();
    &source[start..start + source[start..].find(next).unwrap()]
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
struct Oracle {
    lua: Lua,
    helper: Table,
}
impl Oracle {
    fn new(jit_enabled: bool) -> Self {
        let lua = Lua::new();
        lua.set_memory_limit(256 * 1024 * 1024).unwrap();
        let start = Instant::now();
        lua.set_hook(
            HookTriggers::new().every_nth_instruction(100_000),
            move |_, _| {
                if start.elapsed() > Duration::from_secs(60) {
                    Err(mlua::Error::RuntimeError(
                        "item source oracle deadline exceeded".into(),
                    ))
                } else {
                    Ok(VmState::Continue)
                }
            },
        )
        .unwrap();
        lua.load(if jit_enabled { "jit.on()" } else { "jit.off()" })
            .exec()
            .unwrap();
        lua.globals()
            .set(
                "originalXml",
                lua.load(original("runtime/lua/xml.lua"))
                    .eval::<Table>()
                    .unwrap(),
            )
            .unwrap();
        lua.load("ItemsTabClass={};PassiveSpecClass={};t_insert=table.insert;t_remove=table.remove;s_format=string.format;data={powerStatList={}};legacyClassIdMap={};copyTable=function(t)return t end;ConPrintf=function()end").exec().unwrap();
        for (path, begin, end) in [
            (
                "src/Classes/ItemsTab.lua",
                "function ItemsTabClass:Load(xml, dbFileName)",
                "function ItemsTabClass:Draw(",
            ),
            (
                "src/Classes/ItemsTab.lua",
                "function ItemsTabClass:CreateItemSet(itemSetId, name)",
                "function ItemsTabClass:CopyItemSet(",
            ),
            (
                "src/Classes/ItemsTab.lua",
                "function ItemsTabClass:SetActiveItemSet(itemSetId, deferSync)",
                "-- Equips the given item",
            ),
            (
                "src/Classes/PassiveSpec.lua",
                "function PassiveSpecClass:Load(xml, dbFileName)",
                "function PassiveSpecClass:Save(",
            ),
        ] {
            let full = original(path);
            let from = full.find(begin).unwrap();
            let padded = format!(
                "{}{}",
                "\n".repeat(full[..from].bytes().filter(|v| *v == b'\n').count()),
                section(full, begin, end)
            );
            lua.load(padded)
                .set_name(format!("@{path}"))
                .exec()
                .unwrap();
        }
        let items = original("src/Classes/ItemsTab.lua");
        lua.load(format!("{}\nlocal runeModLines={{{{name='None'}}}}\nfunction original_slot_setup(self)\n{}\nend", section(items,"local baseSlots =", "local runeModLines"), section(items,"\t-- Runes that fit Martial Artist", "\t-- Passive tree dropdown controls"))).set_name("@test-only-original-slot-constructor").exec().unwrap();
        let helper = lua
            .load(include_str!("support/item_source_oracle.lua"))
            .eval()
            .unwrap();
        Self { lua, helper }
    }
    fn parse(&self, xml: &str) -> Table {
        self.helper
            .get::<Function>("parse")
            .unwrap()
            .call(xml)
            .unwrap()
    }
    fn load(&self, node: Table) -> Table {
        self.helper
            .get::<Function>("load")
            .unwrap()
            .call(node)
            .unwrap()
    }
}
fn children(node: &Table, name: &str) -> Vec<Table> {
    node.clone()
        .sequence_values::<Value>()
        .map(Result::unwrap)
        .filter_map(|v| match v {
            Value::Table(t) if t.get::<String>("elem").unwrap() == name => Some(t),
            _ => None,
        })
        .collect()
}
fn strings(node: &Table) -> Vec<String> {
    node.clone()
        .sequence_values::<Value>()
        .map(Result::unwrap)
        .filter_map(|v| match v {
            Value::String(v) => Some(v.to_str().unwrap().to_owned()),
            _ => None,
        })
        .collect()
}
fn attrib(node: &Table, key: &str) -> Option<String> {
    node.get::<Table>("attrib").unwrap().get(key).unwrap()
}
fn events(result: &Table, kind: &str) -> Vec<Table> {
    result
        .get::<Table>("events")
        .unwrap()
        .sequence_values::<Table>()
        .map(Result::unwrap)
        .filter(|t| t.get::<String>("kind").unwrap() == kind)
        .collect()
}

#[test]
fn original_xml_comment_merging_cdata_and_entities_define_distinct_parse_raw_calls() {
    for jit_enabled in [false, true] {
        let o = Oracle::new(jit_enabled);
        for newline in ["\n", "\r\n"] {
            let xml = format!(
                "<Items><Item id=\"1\"> {newline}\tfirst&amp;<!-- ignored -->second {newline}<![CDATA[  third{newline}\t&literal;<!-- stripped even here -->  ]]> fourth&lt;&gt;&apos;&quot; </Item></Items>"
            );
            let item = children(&o.parse(&xml), "Item").remove(0);
            let expected = vec![
                "first&second".to_owned(),
                format!("  third{newline}\t&literal;  "),
                "fourth<>'\"".to_owned(),
            ];
            assert_eq!(strings(&item), expected);
            let loaded = o.load(o.parse(&xml));
            assert!(loaded.get::<bool>("ok").unwrap());
            assert_eq!(
                events(&loaded, "parse_raw")
                    .iter()
                    .map(|v| v.get::<String>("text").unwrap())
                    .collect::<Vec<_>>(),
                expected
            );
            let whitespace = o.parse("<Item> \t <![CDATA[ \r\n ]]> <!--c--> </Item>");
            assert_eq!(whitespace.raw_len(), 0);
        }
    }
}

#[test]
fn original_load_preserves_text_range_interleaving_and_its_legacy_list_order() {
    for jit_enabled in [false, true] {
        let o = Oracle::new(jit_enabled);
        let xml = "<Items><Item id=\"1\">first<ModRange id=\"3\" range=\"0.25\"/><Future/><![CDATA[second]]><ModRange id=\"6\" range=\"0.75\"/><ModRange id=\"11\" range=\"bad\"/></Item></Items>";
        let loaded = o.load(o.parse(xml));
        assert!(loaded.get::<bool>("ok").unwrap());
        let observed = loaded
            .get::<Table>("events")
            .unwrap()
            .sequence_values::<Table>()
            .map(Result::unwrap)
            .map(|t| t.get::<String>("kind").unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            observed,
            [
                "parse_raw",
                "range",
                "parse_raw",
                "range",
                "range",
                "build_mod_list"
            ]
        );
        let ranges = events(&loaded, "range");
        for (r, (list, index, value, parse)) in ranges.iter().zip([
            ("enchant", 1, 0.25_f64, 1),
            ("implicit", 1, 0.75, 2),
            ("explicit", 1, 1.0, 2),
        ]) {
            assert_eq!(r.get::<String>("list").unwrap(), list);
            assert_eq!(r.get::<u32>("index").unwrap(), index);
            assert_eq!(r.get::<f64>("value").unwrap().to_bits(), value.to_bits());
            assert_eq!(r.get::<u32>("parse_ordinal").unwrap(), parse);
        }
        let item = loaded
            .get::<Table>("created")
            .unwrap()
            .get::<Table>(1)
            .unwrap();
        assert!(
            matches!(
                item.get::<Table>("enchantModLines")
                    .unwrap()
                    .get::<Table>(1)
                    .unwrap()
                    .get::<Value>("range")
                    .unwrap(),
                Value::Nil
            ),
            "later ParseRaw reset earlier synthetic range"
        );
        for id in ["0", "-1", "invalid"] {
            let xml = format!("<Items><Item id=\"1\">raw<ModRange id=\"{id}\"/></Item></Items>");
            assert!(
                !o.load(o.parse(&xml)).get::<bool>("ok").unwrap(),
                "source indexes nil for {id}"
            );
        }
        assert!(
            o.load(o.parse("<Items><Item id=\"1\">raw<ModRange id=\"999999\"/></Item></Items>"))
                .get::<bool>("ok")
                .unwrap(),
            "out-of-range instruction falls through"
        );
    }
}

#[test]
fn source_set_keys_duplicates_unknown_slots_and_socket_url_metadata_remain_distinct() {
    for jit_enabled in [false, true] {
        let o = Oracle::new(jit_enabled);
        let xml = "<Items activeItemSet=\"99\"><Item id=\"1\">first</Item><Item id=\"1.0\">second</Item><ItemSet id=\"7\" title=\"old\"><Slot name=\"Amulet\" itemId=\"1\"/></ItemSet><ItemSet id=\"7.0\" title=\"last\" useSecondWeaponSet=\"TRUE\"><Slot name=\"Amulet\" itemId=\"2\" active=\"true\" note=\"arbitrary\"/><Slot name=\"Unknown\" itemId=\"88\"/><SocketIdURL nodeId=\"55\" itemPbURL=\"https://example.invalid/info\"/></ItemSet><ItemSet id=\"nonnumeric\" title=\"allocated ID\"/></Items>";
        let result = o.load(o.parse(xml));
        assert!(result.get::<bool>("ok").unwrap());
        let tab = result.get::<Table>("tab").unwrap();
        assert_eq!(
            tab.get::<Table>("itemOrderList")
                .unwrap()
                .sequence_values::<u32>()
                .map(Result::unwrap)
                .collect::<Vec<_>>(),
            [1, 1]
        );
        assert_eq!(
            tab.get::<Table>("items")
                .unwrap()
                .get::<Table>(1)
                .unwrap()
                .get::<String>("raw")
                .unwrap(),
            "second"
        );
        assert_eq!(
            tab.get::<Table>("itemSetOrderList")
                .unwrap()
                .sequence_values::<u32>()
                .map(Result::unwrap)
                .collect::<Vec<_>>(),
            [7, 7, 1]
        );
        assert_eq!(tab.get::<u32>("activeItemSetId").unwrap(), 7);
        let active = tab.get::<Table>("activeItemSet").unwrap();
        assert_eq!(active.get::<String>("title").unwrap(), "last");
        assert!(!active.get::<bool>("useSecondWeaponSet").unwrap());
        assert_eq!(
            active
                .get::<Table>("Amulet")
                .unwrap()
                .get::<u32>("selItemId")
                .unwrap(),
            2
        );
        assert!(matches!(
            active.get::<Value>("Unknown").unwrap(),
            Value::Nil
        ));
        assert_eq!(
            active
                .get::<Table>(55)
                .unwrap()
                .get::<String>("pbURL")
                .unwrap(),
            "https://example.invalid/info"
        );
        assert!(
            matches!(
                active
                    .get::<Table>(55)
                    .unwrap()
                    .get::<Value>("selItemId")
                    .unwrap(),
                Value::Nil
            ),
            "SocketIdURL does not equip a jewel"
        );
        let legacy = o.load(o.parse(
            "<Items useSecondWeaponSet=\"true\"><Slot name=\"Amulet\" itemId=\"9\"/></Items>",
        ));
        assert!(legacy.get::<bool>("ok").unwrap());
        let tab = legacy.get::<Table>("tab").unwrap();
        assert_eq!(tab.get::<u32>("activeItemSetId").unwrap(), 1);
        assert!(
            tab.get::<Table>("activeItemSet")
                .unwrap()
                .get::<bool>("useSecondWeaponSet")
                .unwrap()
        );
        assert_eq!(
            tab.get::<Table>("activeItemSet")
                .unwrap()
                .get::<Table>("Amulet")
                .unwrap()
                .get::<u32>("selItemId")
                .unwrap(),
            9,
            "SetActiveItemSet copies the legacy live slot back through the same new active set"
        );
    }
}

#[test]
fn authenticated_original_load_methods_and_line_anchors_are_pinned() {
    assert_eq!(
        source::UPSTREAM_REVISION,
        "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
    );
    let o = Oracle::new(false);
    for path in [
        "runtime/lua/xml.lua",
        "src/Classes/ItemsTab.lua",
        "src/Classes/PassiveSpec.lua",
    ] {
        assert_eq!(
            digest(original(path).as_bytes()),
            source::expected_file_sha256(path).unwrap()
        );
    }
    for (class, name, path, first, last) in [
        (
            "ItemsTabClass",
            "Load",
            "src/Classes/ItemsTab.lua",
            1193,
            1320,
        ),
        (
            "ItemsTabClass",
            "Save",
            "src/Classes/ItemsTab.lua",
            1322,
            1411,
        ),
        (
            "ItemsTabClass",
            "CreateItemSet",
            "src/Classes/ItemsTab.lua",
            1571,
            1589,
        ),
        (
            "ItemsTabClass",
            "SetActiveItemSet",
            "src/Classes/ItemsTab.lua",
            1628,
            1670,
        ),
        (
            "PassiveSpecClass",
            "Load",
            "src/Classes/PassiveSpec.lua",
            117,
            250,
        ),
    ] {
        let info = o
            .lua
            .globals()
            .get::<Table>(class)
            .unwrap()
            .get::<Function>(name)
            .unwrap()
            .info();
        assert_eq!(info.source.as_deref(), Some(format!("@{path}").as_str()));
        assert_eq!(info.line_defined, Some(first));
        assert_eq!(info.last_line_defined, Some(last));
        assert_eq!(original(path).lines().nth(last - 1), Some("end"));
    }
}
fn specs(root: &Table) -> Vec<Table> {
    let mut out = children(root, "Spec");
    for tree in children(root, "Tree") {
        out.extend(children(&tree, "Spec"));
    }
    out
}
fn jewel_map(spec: &Table) -> BTreeMap<u32, u32> {
    spec.get::<Table>("jewels")
        .unwrap()
        .pairs::<u32, u32>()
        .map(Result::unwrap)
        .collect()
}
#[test]
fn all_five_immutable_inputs_retain_inventory_sets_range_instructions_and_tree_jewel_ownership() {
    let corpus = repository().join("tests/fixtures/builds/breadth-20260908");
    let index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(corpus.join("index.json")).unwrap()).unwrap();
    for jit_enabled in [false, true] {
        let o = Oracle::new(jit_enabled);
        let (mut items, mut sets, mut ranges, mut jewels) = (0, 0, 0, 0);
        for entry in index["builds"].as_array().unwrap() {
            let path = corpus.join(entry["xml"].as_str().unwrap());
            let before = std::fs::read(&path).unwrap();
            let text = std::str::from_utf8(&before).unwrap();
            assert_eq!(digest(&before), entry["xml_sha256"].as_str().unwrap());
            let root = o.parse(text);
            let containers = children(&root, "Items");
            assert_eq!(containers.len(), 1);
            let node = &containers[0];
            let source_items = children(node, "Item");
            let source_sets = children(node, "ItemSet");
            items += source_items.len();
            sets += source_sets.len();
            ranges += source_items
                .iter()
                .map(|i| children(i, "ModRange").len())
                .sum::<usize>();
            let loaded = o.load(node.clone());
            assert!(
                loaded.get::<bool>("ok").unwrap(),
                "{:?}",
                loaded.get::<Value>("error").unwrap()
            );
            assert_eq!(
                loaded.get::<Table>("created").unwrap().raw_len(),
                source_items.len()
            );
            let expected_raw = source_items.iter().flat_map(strings).collect::<Vec<_>>();
            assert_eq!(
                events(&loaded, "parse_raw")
                    .iter()
                    .map(|r| r.get::<String>("text").unwrap())
                    .collect::<Vec<_>>(),
                expected_raw
            );
            let tab = loaded.get::<Table>("tab").unwrap();
            let order = tab
                .get::<Table>("itemSetOrderList")
                .unwrap()
                .sequence_values::<u32>()
                .map(Result::unwrap)
                .collect::<Vec<_>>();
            assert_eq!(
                order,
                source_sets
                    .iter()
                    .map(|s| attrib(s, "id").unwrap().parse::<u32>().unwrap())
                    .collect::<Vec<_>>()
            );
            for saved in &source_sets {
                let id = attrib(saved, "id").unwrap().parse::<u32>().unwrap();
                let state = tab
                    .get::<Table>("itemSets")
                    .unwrap()
                    .get::<Table>(id)
                    .unwrap();
                assert_eq!(
                    state.get::<String>("title").unwrap(),
                    attrib(saved, "title").unwrap_or("Default".into())
                );
                assert_eq!(
                    state.get::<bool>("useSecondWeaponSet").unwrap(),
                    attrib(saved, "useSecondWeaponSet").as_deref() == Some("true")
                );
                for slot in children(saved, "SocketIdURL") {
                    let node_id = attrib(&slot, "nodeId").unwrap().parse::<u32>().unwrap();
                    let metadata = state.get::<Table>(node_id).unwrap();
                    assert_eq!(
                        metadata.get::<String>("pbURL").unwrap(),
                        attrib(&slot, "itemPbURL").unwrap_or_default()
                    );
                    assert!(matches!(
                        metadata.get::<Value>("selItemId").unwrap(),
                        Value::Nil
                    ));
                }
            }
            for spec in specs(&root) {
                let expected = children(&spec, "Sockets")
                    .into_iter()
                    .flat_map(|s| children(&s, "Socket"))
                    .map(|s| {
                        (
                            attrib(&s, "nodeId").unwrap().parse().unwrap(),
                            attrib(&s, "itemId").unwrap().parse().unwrap(),
                        )
                    })
                    .collect::<BTreeMap<u32, u32>>();
                jewels += expected.len();
                let observed: Table = o
                    .helper
                    .get::<Function>("load_spec")
                    .unwrap()
                    .call((spec, tab.get::<Table>("items").unwrap()))
                    .unwrap();
                assert!(observed.get::<bool>("ok").unwrap());
                assert_eq!(jewel_map(&observed.get::<Table>("spec").unwrap()), expected);
            }
            assert_eq!(std::fs::read(path).unwrap(), before);
        }
        assert_eq!((items, sets, ranges, jewels), (116, 15, 486, 21));
    }
}
#[test]
fn original_passive_jewel_consumer_keeps_specs_separate_and_ignores_stale_references() {
    let o = Oracle::new(false);
    let loaded=o.load(o.parse("<Items><Item id=\"1\">one</Item><Item id=\"2\">two</Item><ItemSet id=\"1\"><SocketIdURL nodeId=\"9\" itemPbURL=\"metadata\"/></ItemSet></Items>"));
    let inventory = loaded
        .get::<Table>("tab")
        .unwrap()
        .get::<Table>("items")
        .unwrap();
    let load = o.helper.get::<Function>("load_spec").unwrap();
    for (text, expected) in [
        (
            "<Spec><Sockets><Socket nodeId=\"9\" itemId=\"1\"/><Socket nodeId=\"9\" itemId=\"2\"/><Socket nodeId=\"10\" itemId=\"404\"/><Socket nodeId=\"11\" itemId=\"0\"/><Socket nodeId=\"bad\" itemId=\"1\"/></Sockets></Spec>",
            BTreeMap::from([(9, 2)]),
        ),
        (
            "<Spec><Sockets><Socket nodeId=\"9\" itemId=\"1\"/></Sockets></Spec>",
            BTreeMap::from([(9, 1)]),
        ),
        (
            "<Spec><Sockets><Unknown nodeId=\"9\" itemId=\"1\"/></Sockets></Spec>",
            BTreeMap::new(),
        ),
    ] {
        let result: Table = load.call((o.parse(text), inventory.clone())).unwrap();
        assert!(result.get::<bool>("ok").unwrap());
        assert_eq!(jewel_map(&result.get::<Table>("spec").unwrap()), expected);
    }
    let missing: Table = load
        .call((
            o.parse("<Spec><Sockets><Socket nodeId=\"9\"/></Sockets></Spec>"),
            inventory,
        ))
        .unwrap();
    assert!(missing.get::<bool>("result").unwrap());
    assert_eq!(missing.get::<Table>("messages").unwrap().raw_len(), 1);
}

// The decisive gate compares projected instructions with original Lua XML tables,
// recursively, including unknown rows. No second hand-written XML parser supplies
// the expected values. Original Load execution above supplies separate observations.
fn compare_projection(node: &ItemSourceNode<'_>, original: &Table, whole: &str) {
    let element = node.element();
    assert_eq!(&whole[element.source_range()], element.source_xml());
    assert!(
        !element.has_namespaces(),
        "namespaces require the separate unknown-context test"
    );
    assert_eq!(element.name(), original.get::<String>("elem").unwrap());
    let attributes = original.get::<Table>("attrib").unwrap();
    assert_eq!(
        element.attributes().len(),
        attributes.clone().pairs::<String, String>().count()
    );
    for attribute in element.attributes() {
        assert_eq!(&whole[attribute.value().range()], attribute.value().raw());
        assert_eq!(
            attribute.value().decoded(),
            attributes.get::<String>(attribute.name()).unwrap()
        );
    }
    let content = node.ordered_content();
    let fragments = content.fragments();
    for (index, fragment) in fragments.iter().enumerate() {
        assert_eq!(&whole[fragment.range()], fragment.raw());
        if index > 0 {
            assert_eq!(fragments[index - 1].range().end, fragment.range().start);
        }
        if fragment.kind() == SourceContentKind::Element {
            let child = &node.children()[fragment.child_index().unwrap()];
            assert_eq!(fragment.range(), child.element().source_range());
            assert_eq!(fragment.raw(), child.element().source_xml());
        }
    }
    assert_eq!(
        content.consumed().len(),
        original.raw_len(),
        "{} consumed array",
        element.name()
    );
    let mut projected_children = 0;
    for (index, entry) in content.consumed().iter().enumerate() {
        match entry {
            PobContentEntry::Text {
                text,
                fragment_indices,
                ..
            } => {
                assert_eq!(
                    text,
                    &original.get::<String>(index + 1).unwrap(),
                    "{} text entry {}",
                    element.name(),
                    index
                );
                assert!(!fragment_indices.is_empty());
                assert!(fragment_indices.iter().all(|i| *i < fragments.len()));
            }
            PobContentEntry::Element { child_index } => {
                assert_eq!(*child_index, projected_children);
                projected_children += 1;
                compare_projection(
                    &node.children()[*child_index],
                    &original.get::<Table>(index + 1).unwrap(),
                    whole,
                );
            }
        }
    }
    assert_eq!(projected_children, node.children().len());
}
fn projection_nodes(node: &ItemSourceNode<'_>, kind: ItemSourceKind) -> usize {
    usize::from(node.kind() == kind)
        + node
            .children()
            .iter()
            .map(|n| projection_nodes(n, kind))
            .sum::<usize>()
}
fn check_document(o: &Oracle, text: &str) {
    let projected = item_source::project_xml(text).unwrap();
    assert_eq!(projected.source_xml(), text);
    assert_eq!(projected.source_sha256(), digest(text.as_bytes()));
    let root = o.parse(text);
    let items = children(&root, "Items");
    assert_eq!(projected.containers().len(), items.len());
    for (node, source) in projected.containers().iter().zip(items) {
        compare_projection(node, &source, text);
    }
    let tree_sources = root
        .sequence_values::<Value>()
        .map(Result::unwrap)
        .filter_map(|v| match v {
            Value::Table(t)
                if matches!(t.get::<String>("elem").unwrap().as_str(), "Tree" | "Spec") =>
            {
                Some(t)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(projected.trees().len(), tree_sources.len());
    for (node, source) in projected.trees().iter().zip(tree_sources) {
        compare_projection(node, &source, text);
    }
}
#[test]
fn portable_projection_matches_original_xml_for_all_corpus_content_and_source_owners() {
    let corpus = repository().join("tests/fixtures/builds/breadth-20260908");
    let index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(corpus.join("index.json")).unwrap()).unwrap();
    for jit_enabled in [false, true] {
        let o = Oracle::new(jit_enabled);
        let (mut items, mut sets, mut ranges, mut jewels) = (0, 0, 0, 0);
        for entry in index["builds"].as_array().unwrap() {
            let bytes = std::fs::read(corpus.join(entry["xml"].as_str().unwrap())).unwrap();
            assert_eq!(digest(&bytes), entry["xml_sha256"].as_str().unwrap());
            let text = std::str::from_utf8(&bytes).unwrap();
            check_document(&o, text);
            let projected = item_source::project_xml(text).unwrap();
            for container in projected.containers() {
                items += projection_nodes(container, ItemSourceKind::Item);
                sets += projection_nodes(container, ItemSourceKind::ItemSet);
                ranges += projection_nodes(container, ItemSourceKind::ModRange);
                assert_eq!(
                    projection_nodes(container, ItemSourceKind::Socket),
                    0,
                    "passive jewel assignment cannot be owned by Items"
                );
            }
            for tree in projected.trees() {
                jewels += projection_nodes(tree, ItemSourceKind::Socket);
                assert_eq!(projection_nodes(tree, ItemSourceKind::Item), 0);
            }
        }
        assert_eq!((items, sets, ranges, jewels), (116, 15, 486, 21));
    }
}
#[test]
fn portable_projection_matches_fragmented_original_instructions_and_keeps_unknown_duplicates() {
    for jit_enabled in [false, true] {
        let o = Oracle::new(jit_enabled);
        for payload in [
            " α&amp;β<!-- join -->γ ",
            " \u{a0}not ASCII whitespace\u{a0} ",
            "first<?probe ignored?>second",
            "<![CDATA[one]]><![CDATA[two]]>",
            "first<!--a--><!--b-->second<![CDATA[ \r\n ]]>third",
            "<![CDATA[<!--unterminated comment text]]>",
        ] {
            check_document(
                &o,
                &format!(
                    "<PathOfBuilding2><Items><Item id=\"1\">{payload}</Item></Items></PathOfBuilding2>"
                ),
            );
        }
        for newline in ["\n", "\r\n"] {
            let text = format!(
                "<PathOfBuilding2><Items activeItemSet=\"bad\"><Item id=\"1\"> {newline}first&amp;<!--c-->second<ModRange id=\"0\" range=\"nan\"/><![CDATA[  third{newline}&raw;<!--stripped-->  ]]><Future note=\" x{newline}\ty &quot;&apos;&lt;&gt;&amp;\"/>tail<?probe ignored?><![CDATA[   ]]></Item><Item id=\"1.0\">duplicate</Item><ItemSet id=\"7\"><Slot name=\"Amulet\" itemId=\"1\"/><Slot name=\"Amulet\" itemId=\"2\"/><SocketIdURL nodeId=\"9\" itemPbURL=\"only metadata\"/></ItemSet><ItemSet id=\"7.0\"/><ItemSet id=\"named\"/><TradeSearchWeights><FutureWeight stat=\"unknown\" weightMult=\"wrong\"/></TradeSearchWeights></Items><Tree activeSpec=\"2\"><Spec title=\"A\"><Sockets><Socket nodeId=\"9\" itemId=\"1\"/></Sockets></Spec><Spec title=\"B\"><Sockets><Socket nodeId=\"9\" itemId=\"2\"/><Socket nodeId=\"9\" itemId=\"3\"/></Sockets></Spec></Tree><Spec title=\"legacy\"><Sockets><Socket nodeId=\"9\" itemId=\"4\"/></Sockets></Spec></PathOfBuilding2>"
            );
            check_document(&o, &text);
            let projection = item_source::project_xml(&text).unwrap();
            let container = &projection.containers()[0];
            assert_eq!(
                container
                    .children()
                    .iter()
                    .filter(|n| n.kind() == ItemSourceKind::ItemSet)
                    .count(),
                3
            );
            assert_eq!(projection.trees().len(), 2);
            assert_eq!(projection.trees()[1].kind(), ItemSourceKind::Spec);
            let item = &container.children()[0];
            let kinds = item
                .ordered_content()
                .fragments()
                .iter()
                .map(|f| f.kind())
                .collect::<Vec<_>>();
            assert!(kinds.contains(&SourceContentKind::Comment));
            assert!(kinds.contains(&SourceContentKind::ProcessingInstruction));
            assert!(kinds.contains(&SourceContentKind::Cdata));
            let text_types = item
                .ordered_content()
                .consumed()
                .iter()
                .filter_map(|e| match e {
                    PobContentEntry::Text { text_kind, .. } => Some(*text_kind),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(
                text_types,
                [
                    PobTextKind::Ordinary,
                    PobTextKind::Cdata,
                    PobTextKind::Ordinary
                ]
            );
            assert_eq!(
                item.children()[0].source_use(),
                ItemSourceUse::ModifierRangeInstruction
            );
            assert_eq!(item.children()[1].kind(), ItemSourceKind::Unknown);
            assert!(
                !container.diagnostics().is_empty()
                    || container
                        .children()
                        .iter()
                        .any(|n| !n.diagnostics().is_empty())
            );
        }
    }
}
#[test]
fn projection_namespaces_stay_unknown_and_lossy_xml_syntax_is_rejected() {
    let text = "<PathOfBuilding2 xmlns:x=\"urn:test\"><Items><Item id=\"1\">raw<x:ModRange id=\"1\" range=\"0.5\"/></Item></Items><x:Items><x:Item id=\"2\">namespaced</x:Item></x:Items></PathOfBuilding2>";
    let projected = item_source::project_xml(text).unwrap();
    assert_eq!(projected.source_xml(), text);
    for container in projected.containers() {
        assert!(container.element().has_namespaces());
        assert_eq!(container.source_use(), ItemSourceUse::NamespaceUnknown);
        assert!(!container.diagnostics().is_empty());
    }
    let o = Oracle::new(false);
    let root = o.parse(text);
    assert_eq!(children(&root, "Items").len(), 1);
    assert_eq!(
        children(&root, "x:Items").len(),
        1,
        "original reader does not resolve namespaces"
    );
    for body in [
        "<Item id =\"1\">raw</Item>",
        "<Item id=\"1\" note=\">\">raw</Item>",
        "<Item id=\"1\">&#10;</Item>",
        "<Item id=\"1\">&#x41;</Item>",
        "<Item id=\"1\" id=\"2\">raw</Item>",
        "<Item id=\"1\">raw</Oops>",
    ] {
        let text = format!("<PathOfBuilding2><Items>{body}</Items></PathOfBuilding2>");
        assert!(item_source::project_xml(&text).is_err(), "{body}");
    }
}
