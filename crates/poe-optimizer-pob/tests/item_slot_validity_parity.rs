//! Complete original IsItemValidForSlot comparisons, distinct from full ItemsTab.Load.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/item_slot_validity_parity.rs"]
mod original;
use mlua::{Function, Lua, Table};
const HELPER: &str = include_str!("support/item_slot_validity_source.lua");
const MECHANICS: &str = r#"
local ItemsTab={}
function ItemsTab:IsItemValidForSlot(item,slotName,itemSet,flagState)
    if slotName=='Weapon' then return item.base.tags.onehand end
end
common={classes={ItemsTab=ItemsTab}}
local shared={onehand=0}
local item={type='Mace',rarity='NORMAL',baseName='Test',base={type='Mace',tags=shared}}
local set={id=1,['Weapon 1']={selItemId=1}}
build={itemsTab=setmetatable({items={[1]=item},itemSets={[1]=set},activeItemSet=set,
    itemOrderList={1},itemSetOrderList={1},slots={['Weapon 1']={}}},{__index=ItemsTab}),
    spec={tree={nodes={[1]={sinister=false}}},nodes={}}}
build.itemsTab.build=build
return item
"#;
#[test]
fn source_binding_retains_method_and_joint_set_alias_without_a_hook() {
    let lua = unsafe { Lua::unsafe_new() };
    let module: Table = lua.load(HELPER).eval().unwrap();
    let item: Table = lua.load(MECHANICS).eval().unwrap();
    let bound: Table = module
        .raw_get::<Function>("bind")
        .unwrap()
        .call(())
        .unwrap();
    let method: Function = bound.raw_get("target").unwrap();
    let receiver: Table = bound.raw_get("receiver").unwrap();
    let result: mlua::MultiValue = method.call((receiver, item, "Weapon")).unwrap();
    assert_eq!(result.len(), 1);
    assert!(matches!(
        result.front(),
        Some(mlua::Value::Integer(0) | mlua::Value::Number(0.0))
    ));
    let projected: Table = bound
        .raw_get::<Function>("projection")
        .unwrap()
        .call(())
        .unwrap();
    assert_eq!(
        projected.raw_get::<Table>("activeItemSet").unwrap(),
        projected
            .raw_get::<Table>("itemSets")
            .unwrap()
            .raw_get::<Table>(1)
            .unwrap()
    );
    assert!(matches!(
        lua.load("return debug.gethook()")
            .eval::<mlua::Value>()
            .unwrap(),
        mlua::Value::Nil
    ));
}
#[test]
fn source_binding_rejects_replaced_method_and_unrepresented_retained_value() {
    let lua = unsafe { Lua::unsafe_new() };
    let module: Table = lua.load(HELPER).eval().unwrap();
    let item: Table = lua.load(MECHANICS).eval().unwrap();
    let bound: Table = module
        .raw_get::<Function>("bind")
        .unwrap()
        .call(())
        .unwrap();
    item.raw_get::<Table>("base")
        .unwrap()
        .raw_get::<Table>("tags")
        .unwrap()
        .raw_set("unknown", lua.create_function(|_, ()| Ok(())).unwrap())
        .unwrap();
    assert!(
        bound
            .raw_get::<Function>("projection")
            .unwrap()
            .call::<Table>(())
            .is_err()
    );
    lua.load("common.classes.ItemsTab.IsItemValidForSlot=function()return true end")
        .exec()
        .unwrap();
    assert!(
        bound
            .raw_get::<Function>("verify")
            .unwrap()
            .call::<()>(())
            .is_err()
    );
}

#[test]
fn source_projection_rejects_metatable_fields_without_invoking_them() {
    let lua = unsafe { Lua::unsafe_new() };
    let module: Table = lua.load(HELPER).eval().unwrap();
    lua.load(MECHANICS).exec().unwrap();
    let bound: Table = module
        .raw_get::<Function>("bind")
        .unwrap()
        .call(())
        .unwrap();
    lua.load("field_reads=0; setmetatable(build.itemsTab.items[1].base,{__index=function()field_reads=field_reads+1;return true end})").exec().unwrap();
    assert!(
        bound
            .raw_get::<Function>("projection")
            .unwrap()
            .call::<Table>(())
            .is_err()
    );
    assert_eq!(lua.globals().raw_get::<u64>("field_reads").unwrap(), 0);
}

#[test]
fn source_projection_resolves_exact_tree_node_inheritance_and_raw_overrides() {
    let lua = unsafe { Lua::unsafe_new() };
    let module: Table = lua.load(HELPER).eval().unwrap();
    lua.load(MECHANICS).exec().unwrap();
    let effective: Table = lua
        .load(
            r#"
local treeNode={sinister=true,containJewelSocket='base socket',charmSocket=false,
    expansionJewel={size=2}}
treeNode.__index=treeNode
build.spec.tree.nodes[1]=treeNode
local effective=setmetatable({sinister=false,charmSocket=0},treeNode)
build.spec.nodes[1]=effective
build.spec.nodes[99]={containJewelSocket=true}
return effective
"#,
        )
        .eval()
        .unwrap();
    let bound: Table = module
        .raw_get::<Function>("bind")
        .unwrap()
        .call(())
        .unwrap();
    let projected: Table = bound
        .raw_get::<Function>("projection")
        .unwrap()
        .call(())
        .unwrap();
    let nodes: Table = projected.raw_get("specNodes").unwrap();
    let actual: Table = nodes.raw_get(1).unwrap();
    for field in ["sinister", "containJewelSocket", "charmSocket"] {
        assert_eq!(
            actual.raw_get::<mlua::Value>(field).unwrap(),
            effective.get::<mlua::Value>(field).unwrap()
        );
    }
    assert!(!actual.raw_get::<bool>("sinister").unwrap());
    assert_eq!(actual.raw_get::<i64>("charmSocket").unwrap(), 0);
    assert_eq!(
        actual.raw_get::<Table>("expansionJewel").unwrap(),
        projected
            .raw_get::<Table>("treeNodes")
            .unwrap()
            .raw_get::<Table>(1)
            .unwrap()
            .raw_get::<Table>("expansionJewel")
            .unwrap()
    );
    assert!(
        nodes
            .raw_get::<Table>(99)
            .unwrap()
            .raw_get::<bool>("containJewelSocket")
            .unwrap()
    );
    bound
        .raw_get::<Function>("verify")
        .unwrap()
        .call::<()>(())
        .unwrap();
}

#[test]
fn source_projection_rejects_unknown_node_inheritance_without_callbacks() {
    for alteration in [
        "local copy={sinister=true}; copy.__index=copy; copy.__eq=trap; original.__eq=trap; setmetatable(build.spec.nodes[1],copy)",
        "local other={sinister=true}; other.__index=other; build.spec.tree.nodes[2]=other; setmetatable(build.spec.nodes[1],other)",
        "original.__index=trap",
        "setmetatable(original,{__index=trap})",
        "setmetatable(build.spec.nodes,{__index=trap})",
    ] {
        let lua = unsafe { Lua::unsafe_new() };
        let module: Table = lua.load(HELPER).eval().unwrap();
        lua.load(MECHANICS).exec().unwrap();
        lua.load(
            r#"
field_reads=0
function trap() field_reads=field_reads+1; return true end
original=build.spec.tree.nodes[1]
original.__index=original
build.spec.nodes[1]=setmetatable({},original)
"#,
        )
        .exec()
        .unwrap();
        let bound: Table = module
            .raw_get::<Function>("bind")
            .unwrap()
            .call(())
            .unwrap();
        lua.load(alteration).exec().unwrap();
        assert!(
            bound
                .raw_get::<Function>("projection")
                .unwrap()
                .call::<Table>(())
                .is_err(),
            "{alteration}"
        );
        assert_eq!(
            lua.globals().raw_get::<u64>("field_reads").unwrap(),
            0,
            "{alteration}"
        );
    }
}

#[test]
fn source_projection_rechecks_retained_node_context_identity() {
    let lua = unsafe { Lua::unsafe_new() };
    let module: Table = lua.load(HELPER).eval().unwrap();
    lua.load(MECHANICS).exec().unwrap();
    let bound: Table = module
        .raw_get::<Function>("bind")
        .unwrap()
        .call(())
        .unwrap();
    lua.load("build.itemsTab.build={}").exec().unwrap();
    assert!(
        bound
            .raw_get::<Function>("verify")
            .unwrap()
            .call::<()>(())
            .is_err()
    );
    lua.load("build.itemsTab.build=build; build.spec.nodes={}")
        .exec()
        .unwrap();
    assert!(
        bound
            .raw_get::<Function>("verify")
            .unwrap()
            .call::<()>(())
            .is_err()
    );
}

// mlua requires Arc<Error> in these variants even for a single-threaded Lua host.
#[allow(clippy::arc_with_non_send_sync)]
#[test]
fn source_error_classification_accepts_only_runtime_callback_chains() {
    fn callback(cause: mlua::Error) -> mlua::Error {
        mlua::Error::CallbackError {
            traceback: "test-only callback trace".into(),
            cause: std::sync::Arc::new(cause),
        }
    }
    let runtime = mlua::Error::RuntimeError("declared source failure".into());
    assert!(original::is_source_runtime_error(&runtime));
    assert!(original::is_source_runtime_error(&callback(callback(
        runtime.clone()
    ))));
    for error in [
        mlua::Error::MemoryError("test-only memory failure".into()),
        mlua::Error::StackError,
        mlua::Error::CallbackDestructed,
        mlua::Error::FromLuaConversionError {
            from: "nil",
            to: "Table".into(),
            message: None,
        },
        mlua::Error::SyntaxError {
            message: "test-only syntax failure".into(),
            incomplete_input: false,
        },
        mlua::Error::BadArgument {
            to: None,
            pos: 1,
            name: None,
            cause: std::sync::Arc::new(runtime.clone()),
        },
        mlua::Error::WithContext {
            context: "non-callback wrapper".into(),
            cause: std::sync::Arc::new(runtime),
        },
    ] {
        assert!(!original::is_source_runtime_error(&error), "{error:?}");
        assert!(!original::is_source_runtime_error(&callback(callback(
            error
        ))));
    }
}

#[test]
fn all_five_original_item_slot_validity_matches_native_component() {
    original::run();
}
