//! Direct post-import observations of actual flask Items, never population traversal.
use super::*;

const MAX_FLASKS: usize = 128;
const MAX_SLOTS: usize = 8;
const MAX_CALLS: usize = 256;

pub(super) fn observe(_lua: &Lua, module: &Table, case: &str, xml: &str, execute: bool) -> Json {
    let bound = bind_original(module);
    let project: Function = bound.raw_get("projection").unwrap();
    let before: Table = project.call(()).unwrap();
    let before_graph = source_graph(&[LuaValue::Table(before.clone())]);
    if !execute {
        return json!({"initial_read_set":before_graph,"source_only_control":true});
    }
    let method: Function = bound.raw_get("target").unwrap();
    let receiver: Table = bound.raw_get("receiver").unwrap();
    let items: Table = bound.raw_get("items").unwrap();
    let projected_items: Table = before.raw_get("items").unwrap();
    let mut ids = bound
        .raw_get::<Table>("item_ids")
        .unwrap()
        .sequence_values::<i64>()
        .collect::<mlua::Result<Vec<_>>>()
        .unwrap();
    ids.sort_unstable();
    assert!(ids.len() <= 4096);
    let document = roxmltree::Document::parse(xml).unwrap();
    let mut authored = BTreeMap::new();
    for (ordinal, node) in document
        .descendants()
        .filter(|node| {
            node.has_tag_name("Item") && node.parent().is_some_and(|p| p.has_tag_name("Items"))
        })
        .enumerate()
    {
        assert!(ordinal < 4096);
        let id = node.attribute("id").unwrap().parse::<i64>().unwrap();
        assert!(
            authored
                .insert(id, (ordinal, hash(xml[node.range()].as_bytes())))
                .is_none(),
            "original authored Item ID must identify exactly one retained Item"
        );
    }
    assert_eq!(
        ids.iter().copied().collect::<BTreeSet<_>>(),
        authored.keys().copied().collect::<BTreeSet<_>>()
    );
    let mut slots = bound
        .raw_get::<Table>("slots")
        .unwrap()
        .sequence_values::<String>()
        .collect::<mlua::Result<Vec<_>>>()
        .unwrap();
    slots.retain(|slot| {
        slot == "Flask"
            || slot.strip_prefix("Flask ").is_some_and(|suffix| {
                !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit())
            })
    });
    slots.sort();
    assert!(!slots.is_empty() && slots.len() <= MAX_SLOTS);
    let mut flask_items = Vec::new();
    let mut results = Vec::new();
    for id in &ids {
        let item: Table = items.raw_get(*id).unwrap();
        if !matches!(item.raw_get::<LuaValue>("type").unwrap(), LuaValue::String(ref value) if value.as_bytes().as_ref() == b"Flask")
        {
            continue;
        }
        assert!(flask_items.len() < MAX_FLASKS);
        let (ordinal, authored_sha) = &authored[id];
        let raw_type: LuaValue = item.raw_get("type").unwrap();
        let base_name: LuaValue = item.raw_get("baseName").unwrap();
        flask_items.push(json!({
            "item_id":id,"authored_item_ordinal_zero_based":ordinal,
            "authored_item_xml_sha256":authored_sha,
            "raw_field_names":["type","baseName"],
            "raw_field_types":[raw_type.type_name(),base_name.type_name()],
            "raw_field_values":source_graph(&[raw_type,base_name]),
            "declared_item_read_set":source_graph(&[LuaValue::Table(projected_items.raw_get(*id).unwrap())])
        }));
        for slot in &slots {
            assert!(results.len() < MAX_CALLS);
            assert_eq!(
                items.raw_get::<Table>(*id).unwrap().to_pointer(),
                item.to_pointer()
            );
            let outcome = match method.call::<MultiValue>((
                receiver.clone(),
                item.clone(),
                slot.as_str(),
                LuaValue::Nil,
                LuaValue::Nil,
            )) {
                Ok(pack) => {
                    // This complete source branch returns true or falls through;
                    // a different result is evidence that the probe's scope changed.
                    assert!(
                        pack.len() <= 1
                            && pack
                                .iter()
                                .all(|v| matches!(v, LuaValue::Boolean(_) | LuaValue::Nil))
                    );
                    let values = pack.into_iter().collect::<Vec<_>>();
                    json!({"status":"returned","return_count":values.len(),"result":source_graph(&values)})
                }
                Err(error) => {
                    assert!(
                        is_source_runtime_error(&error),
                        "source probe host failure: {error:?}"
                    );
                    let text = error.to_string();
                    assert!(
                        text.len() <= 4096
                            && !text.contains("slot validity observation:")
                            && !text.contains("deadline")
                            && !text.contains("budget"),
                        "source probe observer failure: {text}"
                    );
                    assert!(
                        (2603..=2687).any(|line| text.contains(&format!("ItemsTab.lua:{line}:"))),
                        "source failure must be attributed to the complete retained method: {text}"
                    );
                    json!({"status":"source_error","message":text})
                }
            };
            assert_eq!(
                items.raw_get::<Table>(*id).unwrap().to_pointer(),
                item.to_pointer()
            );
            results.push(
                json!({"item_id":id,"slot":slot,"item_set":"nil (actual active set fallback)",
                "flag_state":"nil","outcome":outcome}),
            );
        }
    }
    bound
        .raw_get::<Function>("verify")
        .unwrap()
        .call::<()>(())
        .unwrap();
    let after: Table = project.call(()).unwrap();
    assert_eq!(
        before_graph,
        source_graph(&[LuaValue::Table(after)]),
        "flask probes changed declared read set"
    );
    assert_eq!(results.len(), flask_items.len() * slots.len());
    // These are diagnosed native input pairs, not source expected return values
    // or evidence about the original population traversal/active-set timing.
    let diagnosed_id = match case {
        "build-02.xml" => Some(12),
        "build-04.xml" | "build-05.xml" => Some(7),
        _ => None,
    };
    if let Some(id) = diagnosed_id {
        assert!(
            flask_items.iter().any(|item| item["item_id"] == id),
            "diagnosed authored flask absent: {case}/{id}"
        );
        assert!(
            results
                .iter()
                .any(|row| row["item_id"] == id && row["slot"] == "Flask 1"),
            "diagnosed Item/slot pair absent: {case}/{id}/Flask 1"
        );
    }
    let active_set_id: LuaValue = receiver.raw_get("activeItemSetId").unwrap();
    json!({"initial_read_set":before_graph,"item_count":ids.len(),"flask_count":flask_items.len(),
        "diagnosed_native_pair":diagnosed_id.map(|id|json!({"item_id":id,"slot":"Flask 1"})),
        "actual_post_import_active_set_id":source_graph(&[active_set_id]),
        "flask_items":flask_items,"runtime_flask_slots":slots,"calls":results.len(),"results":results,
        "scope":{"complete_original_method":true,"post_import_context":true,"direct_supplied_inputs":true,
            "source_only":true,"native_parity":false,"source_population_order_observed":false,
            "method_replaced":false,"new_call_hook":false,"exact_return_pack":true,
            "original_item_identity_retained":true,"declared_read_set_unchanged":true,
            "whole_original_object_graph":false,"enumeration_order":"sorted actual Item IDs and runtime flask slot labels"}})
}
