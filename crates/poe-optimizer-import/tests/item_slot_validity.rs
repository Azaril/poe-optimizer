//! Source-ordered component checks; caller contexts are explicit, not equipment activation.
use poe_optimizer_data::{
    game_data::{GameDataSnapshot, bundled_snapshot},
    item_assembly::ItemSlotValidityPolicy,
    item_loading::{ItemMetadataTable as T, ItemMetadataValue as M},
};
use poe_optimizer_import::{
    item_loading::{
        BuiltinItemLoadProvider, ItemLoadMachine,
        assembly::{AssemblyError, AssemblyErrorKind},
    },
    item_slot_validity::*,
};
use std::{collections::BTreeMap, sync::OnceLock};
fn data() -> &'static GameDataSnapshot {
    static DATA: OnceLock<GameDataSnapshot> = OnceLock::new();
    DATA.get_or_init(|| bundled_snapshot().unwrap())
}
fn policy() -> ItemSlotValidityPolicy {
    data().item_assembly().policy().slot_validity.clone()
}
fn text(s: &str) -> M {
    M::Text(s.into())
}
fn table(fields: impl IntoIterator<Item = (&'static str, M)>) -> T {
    T {
        fields: fields.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        indexed: BTreeMap::new(),
    }
}
fn item(kind: &str, tags: T) -> T {
    table([
        ("type", text(kind)),
        ("rarity", text("NORMAL")),
        ("baseName", text("test base")),
        (
            "base",
            M::Table(table([("type", text(kind)), ("tags", M::Table(tags))])),
        ),
    ])
}
fn selected(primary: &str) -> T {
    T {
        fields: [(
            primary.into(),
            M::Table(table([("selItemId", M::Number(1.0))])),
        )]
        .into(),
        indexed: BTreeMap::new(),
    }
}
struct Context<'a> {
    active: Value<'a>,
    tree: Value<'a>,
    effective: Value<'a>,
    inventory: Value<'a>,
    environment: bool,
    flags: Value<'a>,
    events: Vec<String>,
    fail_flag: Option<String>,
}
impl<'a> Context<'a> {
    fn empty() -> Self {
        Self {
            active: Value::Nil,
            tree: Value::Nil,
            effective: Value::Nil,
            inventory: Value::Nil,
            environment: false,
            flags: Value::Nil,
            events: Vec::new(),
            fail_flag: None,
        }
    }
}
impl<'a> SlotValidityContext<'a> for Context<'a> {
    fn active_item_set(&mut self) -> Result<Value<'a>> {
        self.events.push("active".into());
        Ok(self.active)
    }
    fn tree_node(&mut self, key: Value<'_>) -> Result<Value<'a>> {
        self.events.push(format!("tree:{key:?}"));
        Ok(self.tree)
    }
    fn effective_node(&mut self, key: Value<'_>) -> Result<Value<'a>> {
        self.events.push(format!("effective:{key:?}"));
        Ok(self.effective)
    }
    fn inventory_item(&mut self, key: Value<'_>) -> Result<Value<'a>> {
        self.events.push(format!("inventory:{key:?}"));
        Ok(self.inventory)
    }
    fn has_calculation_environment(&mut self) -> Result<bool> {
        self.events.push("environment".into());
        Ok(self.environment)
    }
    fn flag(&mut self, name: &str) -> Result<Value<'a>> {
        self.events.push(format!("flag:{name}"));
        if self.fail_flag.as_deref() == Some(name) {
            return Err(AssemblyError::source("directed flag failure"));
        }
        self.flags.field(name)
    }
}
fn run<'a>(
    p: &ItemSlotValidityPolicy,
    item: Value<'a>,
    slot: &'a str,
    set: Value<'a>,
    flags: Value<'a>,
    ctx: &mut impl SlotValidityContext<'a>,
) -> Result<SlotValidityResult<'a>> {
    is_item_valid_for_slot(
        p,
        SlotValidityRequest {
            item,
            slot_name: slot,
            item_set: set,
            flag_state: flags,
        },
        ctx,
        SlotValidityLimits::default(),
    )
}
fn scalar(result: SlotValidityResult<'_>, expected: Value<'_>) {
    let SlotValidityResult::Value(actual) = result else {
        panic!("expected one result")
    };
    assert!(actual.same_identity(expected), "{actual:?} != {expected:?}");
}
#[test]
fn active_set_lookup_precedes_patterns_and_unused_context_stays_lazy() {
    struct Absent;
    impl SlotValidityContext<'_> for Absent {}
    let mut p = policy();
    p.slot_pattern = "[".into();
    let err = run(
        &p,
        Value::Nil,
        "anything",
        Value::Nil,
        Value::Nil,
        &mut Absent,
    )
    .unwrap_err();
    assert_eq!(err.kind, AssemblyErrorKind::Unsupported);
    assert!(err.message.contains("active item set"));
    let set = T::default();
    assert_eq!(
        run(
            &p,
            Value::Nil,
            "anything",
            Value::table(&set),
            Value::Nil,
            &mut Absent
        )
        .unwrap_err()
        .kind,
        AssemblyErrorKind::Source
    );
    let ordinary = item("Ring", T::default());
    scalar(
        run(
            &policy(),
            Value::table(&ordinary),
            "Ring 1",
            Value::table(&set),
            Value::Nil,
            &mut Absent,
        )
        .unwrap(),
        Value::Boolean(true),
    );
}
#[test]
fn jewels_read_node_fallback_before_item_and_preserve_contained_subtype_nil() {
    let p = policy();
    let set = T::default();
    let mut ctx = Context::empty();
    scalar(
        run(
            &p,
            Value::Nil,
            "Jewel 73",
            Value::table(&set),
            Value::Nil,
            &mut ctx,
        )
        .unwrap(),
        Value::Boolean(false),
    );
    assert_eq!(ctx.events, ["tree:Number(73.0)", "effective:Number(73.0)"]);
    let node = table([("containJewelSocket", M::Boolean(true))]);
    let mut jewel = item("Jewel", T::default());
    jewel.fields.remove("base");
    ctx.tree = Value::table(&node);
    ctx.events.clear();
    scalar(
        run(
            &p,
            Value::table(&jewel),
            "Jewel 73",
            Value::table(&set),
            Value::Nil,
            &mut ctx,
        )
        .unwrap(),
        Value::Boolean(true),
    );
    assert_eq!(ctx.events.len(), 1);
    // A context borrows the same lifetime as its request; each edited fixture
    // gets a fresh context, preserving the original read-order assertions.
    let mut ctx = Context::empty();
    ctx.tree = Value::table(&node);
    jewel.fields.insert(
        "base".into(),
        M::Table(table([("subType", M::Boolean(false))])),
    );
    scalar(
        run(
            &p,
            Value::table(&jewel),
            "Jewel 73",
            Value::table(&set),
            Value::Nil,
            &mut ctx,
        )
        .unwrap(),
        Value::Boolean(false),
    );
    assert_eq!(ctx.events.len(), 1);
    let mut ctx = Context::empty();
    ctx.tree = Value::table(&node);
    jewel.fields.insert("rarity".into(), text("UNIQUE"));
    jewel.fields.insert("base".into(), M::Number(7.0));
    scalar(
        run(
            &p,
            Value::table(&jewel),
            "Jewel 73",
            Value::table(&set),
            Value::Nil,
            &mut ctx,
        )
        .unwrap(),
        Value::Boolean(false),
    );
}
#[test]
fn cluster_comparison_is_typed_and_charm_short_circuit_is_preserved() {
    let p = policy();
    let set = T::default();
    let mut jewel = item("Jewel", T::default());
    jewel.fields.insert(
        "clusterJewel".into(),
        M::Table(table([("sizeIndex", text("10"))])),
    );
    let node = table([("expansionJewel", M::Table(table([("size", text("2"))])))]);
    let mut ctx = Context::empty();
    ctx.tree = Value::table(&node);
    scalar(
        run(
            &p,
            Value::table(&jewel),
            "Jewel 1",
            Value::table(&set),
            Value::Nil,
            &mut ctx,
        )
        .unwrap(),
        Value::Boolean(true),
    );
    let node = table([(
        "expansionJewel",
        M::Table(table([("size", M::Number(1.0))])),
    )]);
    ctx.tree = Value::table(&node);
    assert_eq!(
        run(
            &p,
            Value::table(&jewel),
            "Jewel 1",
            Value::table(&set),
            Value::Nil,
            &mut ctx
        )
        .unwrap_err()
        .kind,
        AssemblyErrorKind::Source
    );
    let node = table([("charmSocket", M::Boolean(true))]);
    let mut ctx = Context::empty();
    ctx.tree = Value::table(&node);
    jewel
        .fields
        .insert("base".into(), M::Table(table([("subType", text("Charm"))])));
    scalar(
        run(
            &p,
            Value::table(&jewel),
            "Jewel 1",
            Value::table(&set),
            Value::Nil,
            &mut ctx,
        )
        .unwrap(),
        Value::Boolean(true),
    );
}
#[test]
fn flask_failed_route_does_not_fall_through_to_equal_type_and_subtypes_are_lazy() {
    let p = policy();
    let set = T::default();
    let mut flask = item("Flask", T::default());
    let mut ctx = Context::empty();
    assert!(matches!(
        run(
            &p,
            Value::table(&flask),
            "Flask 1",
            Value::table(&set),
            Value::Nil,
            &mut ctx
        )
        .unwrap(),
        SlotValidityResult::NoValues
    ));
    assert!(ctx.events.is_empty());
    let mut ctx = Context::empty();
    flask
        .fields
        .insert("baseName".into(), text("Ultimate Life Flask"));
    scalar(
        run(
            &p,
            Value::table(&flask),
            "Flask 1",
            Value::table(&set),
            Value::Nil,
            &mut ctx,
        )
        .unwrap(),
        Value::Boolean(true),
    );
    let mut limb = item("custom", T::default());
    limb.fields.insert(
        "base".into(),
        M::Table(table([("subType", text("Transcendent Arm"))])),
    );
    scalar(
        run(
            &p,
            Value::table(&limb),
            "Arm 1",
            Value::table(&set),
            Value::Nil,
            &mut ctx,
        )
        .unwrap(),
        Value::Boolean(true),
    );
    assert!(ctx.events.is_empty());
    let mut ctx = Context::empty();
    limb.fields.remove("base");
    assert_eq!(
        run(
            &p,
            Value::table(&limb),
            "Ring 1",
            Value::table(&set),
            Value::Nil,
            &mut ctx
        )
        .unwrap_err()
        .kind,
        AssemblyErrorKind::Source
    );
    assert!(ctx.events.is_empty());
}
#[test]
fn embedded_parent_restrictions_preserve_false_absence_and_truthy_zero() {
    let p = policy();
    let jewel = item("Jewel", T::default());
    let set = selected("Helmet");
    let mut ctx = Context::empty();
    let parent = table([(
        "canSocketJewelBase",
        M::Table(table([("test base", M::Number(0.0))])),
    )]);
    ctx.inventory = Value::table(&parent);
    scalar(
        run(
            &p,
            Value::table(&jewel),
            "Helmet Jewel Socket 1",
            Value::table(&set),
            Value::Nil,
            &mut ctx,
        )
        .unwrap(),
        Value::Boolean(true),
    );
    assert_eq!(ctx.events, ["inventory:Number(1.0)"]);
    let parent = table([("canSocketJewelBase", M::Boolean(false))]);
    ctx.inventory = Value::table(&parent);
    scalar(
        run(
            &p,
            Value::table(&jewel),
            "Helmet Jewel Socket 1",
            Value::table(&set),
            Value::Nil,
            &mut ctx,
        )
        .unwrap(),
        Value::Boolean(true),
    );
    let parent = table([("canSocketJewelBase", M::Boolean(true))]);
    ctx.inventory = Value::table(&parent);
    assert_eq!(
        run(
            &p,
            Value::table(&jewel),
            "Helmet Jewel Socket 1",
            Value::table(&set),
            Value::Nil,
            &mut ctx
        )
        .unwrap_err()
        .kind,
        AssemblyErrorKind::Source
    );
}
#[test]
fn primary_raw_tag_alias_and_zero_versus_one_nil_results_are_not_normalized() {
    let p = policy();
    let set = T::default();
    let mut ctx = Context::empty();
    let weapon = item(
        "custom",
        table([("onehand", M::Table(table([("marker", M::Number(7.0))])))]),
    );
    let expected = Value::table(&weapon)
        .field("base")
        .unwrap()
        .field("tags")
        .unwrap()
        .field("onehand")
        .unwrap();
    scalar(
        run(
            &p,
            Value::table(&weapon),
            "Weapon 1",
            Value::table(&set),
            Value::Nil,
            &mut ctx,
        )
        .unwrap(),
        expected,
    );
    let weapon = item("custom", table([("onehand", M::Boolean(false))]));
    scalar(
        run(
            &p,
            Value::table(&weapon),
            "Weapon 1",
            Value::table(&set),
            Value::Nil,
            &mut ctx,
        )
        .unwrap(),
        Value::Nil,
    );
    let empty = run(
        &p,
        Value::table(&weapon),
        "Unknown Slot",
        Value::table(&set),
        Value::Nil,
        &mut ctx,
    )
    .unwrap();
    assert_eq!(empty.arity(), 0);
    assert!(!empty.truthy());
    let Value::Table(tag) = expected else {
        panic!()
    };
    let entries = tag.entries().unwrap().collect::<Result<Vec<_>>>().unwrap();
    assert_eq!(entries.len(), 1);
}
#[test]
fn offhand_queries_flags_in_order_before_base_type_and_never_invents_missing_context() {
    let p = policy();
    let set = selected("Weapon 1");
    let bow = item("Bow", T::default());
    let quiver = item("Quiver", T::default());
    let flags = T::default();
    let mut ctx = Context::empty();
    ctx.inventory = Value::table(&bow);
    ctx.environment = true;
    ctx.flags = Value::table(&flags);
    ctx.fail_flag = Some(p.weapon.instruments_of_power.query_name.clone());
    let err = run(
        &p,
        Value::table(&quiver),
        "Weapon 2",
        Value::table(&set),
        Value::Nil,
        &mut ctx,
    )
    .unwrap_err();
    assert_eq!(err.message, "directed flag failure");
    assert_eq!(
        ctx.events,
        vec![
            "inventory:Number(1.0)".to_owned(),
            "inventory:Number(1.0)".to_owned(),
            "environment".to_owned(),
            format!("flag:{}", p.weapon.giants_blood.query_name),
            format!("flag:{}", p.weapon.instruments_of_power.query_name)
        ]
    );
    ctx.events.clear();
    scalar(
        run(
            &p,
            Value::table(&quiver),
            "Weapon 2",
            Value::table(&set),
            Value::table(&flags),
            &mut ctx,
        )
        .unwrap(),
        Value::Boolean(true),
    );
    assert_eq!(ctx.events.len(), 2);
    let malformed = table([("base", M::Number(7.0))]);
    ctx.inventory = Value::table(&malformed);
    assert_eq!(
        run(
            &p,
            Value::table(&quiver),
            "Weapon 2",
            Value::table(&set),
            Value::Nil,
            &mut ctx
        )
        .unwrap_err()
        .message,
        "directed flag failure"
    );
}
#[test]
fn offhand_raw_final_operand_and_special_routes_obey_source_short_circuit() {
    let p = policy();
    let set = selected("Weapon 1");
    let mut ctx = Context::empty();
    let flags = T::default();
    let candidate = item("custom", T::default());
    scalar(
        run(
            &p,
            Value::table(&candidate),
            "Weapon 2",
            Value::table(&set),
            Value::table(&flags),
            &mut ctx,
        )
        .unwrap(),
        Value::Nil,
    );
    let flags = table([("giantsBlood", M::Boolean(false))]);
    scalar(
        run(
            &p,
            Value::table(&candidate),
            "Weapon 2",
            Value::table(&set),
            Value::table(&flags),
            &mut ctx,
        )
        .unwrap(),
        Value::Boolean(false),
    );
    let candidate = item(
        "custom",
        table([("axe", M::Table(table([("marker", M::Number(9.0))])))]),
    );
    let expected = Value::table(&candidate)
        .field("base")
        .unwrap()
        .field("tags")
        .unwrap()
        .field("axe")
        .unwrap();
    scalar(
        run(
            &p,
            Value::table(&candidate),
            "Weapon 2",
            Value::table(&set),
            Value::Nil,
            &mut ctx,
        )
        .unwrap(),
        expected,
    );
    let primary = item("Staff", T::default());
    ctx.inventory = Value::table(&primary);
    let candidate = item("Focus", T::default());
    scalar(
        run(
            &p,
            Value::table(&candidate),
            "Weapon 2",
            Value::table(&set),
            Value::Nil,
            &mut ctx,
        )
        .unwrap(),
        Value::Boolean(true),
    );
}
#[test]
fn changed_policy_compiles_once_and_query_budgets_reset_without_state_mutation() {
    let mut p = policy();
    p.weapon.primary_slots = vec!["Held".into()];
    p.weapon.primary_tags = ["left".into(), "right".into()];
    p.slot_pattern = "^(%a+)#(%d+)$".into();
    let program = SlotValidityProgram::new(&p, SlotValidityLimits::default()).unwrap();
    let clone = program.clone();
    assert!(program.shares_storage_with(&clone));
    assert_eq!(program.policy(), &p);
    let weapon = item("custom", table([("right", M::Number(0.0))]));
    let before = weapon.clone();
    let set = T::default();
    let request = SlotValidityRequest {
        item: Value::table(&weapon),
        slot_name: "Held",
        item_set: Value::table(&set),
        flag_state: Value::Nil,
    };
    let mut ctx = Context::empty();
    for _ in 0..3 {
        scalar(
            program.check(request, &mut ctx).unwrap(),
            Value::Number(0.0),
        );
    }
    assert_eq!(weapon, before);
    let tiny = SlotValidityProgram::new(
        &p,
        SlotValidityLimits {
            max_steps: 1,
            ..SlotValidityLimits::default()
        },
    )
    .unwrap();
    assert_eq!(
        tiny.check(request, &mut ctx).unwrap_err().kind,
        AssemblyErrorKind::Resource
    );
    p.embedded.slot_pattern = "[".into();
    let program = SlotValidityProgram::new(&p, SlotValidityLimits::default()).unwrap();
    scalar(
        program.check(request, &mut ctx).unwrap(),
        Value::Number(0.0),
    );
}
#[test]
fn actual_owned_item_queries_borrow_without_cloning_or_mutating_graph() {
    let mut machine = ItemLoadMachine::new(data().item_loading());
    machine.set_xml_attributes(&[("id".into(), "1".into())].into());
    let mut provider = BuiltinItemLoadProvider::new(data());
    machine
        .apply_text("Rarity: NORMAL\nWooden Club", &mut provider)
        .unwrap();
    machine.finish_load(&mut provider).unwrap();
    let assembled = machine.assembled().unwrap();
    let before = assembled.snapshot().unwrap();
    let set = T::default();
    let mut ctx = Context::empty();
    let program = SlotValidityProgram::new(&policy(), SlotValidityLimits::default()).unwrap();
    let result = program
        .check(
            SlotValidityRequest {
                item: Value::item(assembled),
                slot_name: "Weapon 1",
                item_set: Value::table(&set),
                flag_state: Value::Nil,
            },
            &mut ctx,
        )
        .unwrap();
    assert!(result.truthy());
    assert_eq!(result.arity(), 1);
    assert_eq!(assembled.snapshot().unwrap(), before);
    assert!(ctx.events.is_empty());
}

#[test]
fn preparation_validates_untrusted_policy_and_bounds_all_pattern_storage() {
    let mut p = policy();
    p.weapon.giant_tags.push("x".repeat(4097));
    assert_eq!(
        SlotValidityProgram::new(&p, SlotValidityLimits::default())
            .unwrap_err()
            .kind,
        AssemblyErrorKind::Unsupported
    );
    let p = policy();
    let bytes = poe_optimizer_engine::lua_pattern::LuaPattern::compile(p.slot_pattern.as_bytes())
        .unwrap()
        .compiled_bytes();
    assert_eq!(
        SlotValidityProgram::new(
            &p,
            SlotValidityLimits {
                max_compiled_bytes: bytes,
                ..SlotValidityLimits::default()
            }
        )
        .unwrap_err()
        .kind,
        AssemblyErrorKind::Resource
    );
    for name in ["split", "matchOrPattern", "custom_method"] {
        assert_eq!(
            Value::Text("context").field(name).unwrap_err().kind,
            AssemblyErrorKind::Unsupported
        );
    }
    assert!(matches!(
        Value::Text("context").field("type").unwrap(),
        Value::Nil
    ));
}

#[test]
fn empty_array_identity_uses_the_owner_not_the_shared_dangling_storage_pointer() {
    let first = M::Array(Vec::new());
    let second = M::Array(Vec::new());
    let a = Value::metadata(&first).unwrap();
    let b = Value::metadata(&second).unwrap();
    assert!(a.same_identity(Value::metadata(&first).unwrap()));
    assert!(!a.same_identity(b));
    let Value::Table(array) = a else {
        panic!("array table")
    };
    assert!(array.is_empty());
    assert_eq!(array.entries().unwrap().count(), 0);
    assert!(matches!(a.index(Value::Number(1.0)).unwrap(), Value::Nil));
}
