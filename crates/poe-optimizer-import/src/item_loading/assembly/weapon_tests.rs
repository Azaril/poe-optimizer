use super::*;
use crate::item_loading::{
    AssemblyExecution, BuiltinItemLoadProvider, ItemLoadMachine, ParseOutcome, ParseRequest,
    UnavailableItemLoadProvider,
};
use poe_optimizer_data::{
    game_data::{GameDataSnapshot, bundled_snapshot},
    item_assembly::{
        ItemAssemblyCatalog, ItemAssemblyLocalQuery, ItemAssemblyWeaponDamageKind as Kind,
    },
    item_loading::ItemMetadataValue,
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
        fn assemble_with_trace(&mut self, r: &AssemblyRequest) -> AssemblyExecution {
            self.0 = Some(r.clone());
            AssemblyExecution {
                outcome: DependencyResult::Unavailable("capture input".into()),
                prefix: None,
            }
        }
    }
    let mut machine = ItemLoadMachine::new(data().item_loading());
    let mut capture = Capture::default();
    machine
        .apply_text("Rarity: Normal\nGold Ring", &mut capture)
        .unwrap();
    capture.0.unwrap()
}
type C<'d, 'r> = Context<'d, 'r, UnavailableItemLoadProvider>;
fn setup<'d, 'r>(
    defs: ItemAssemblyDefinitions<'d>,
    r: &'r AssemblyRequest,
    d: &'r mut UnavailableItemLoadProvider,
) -> (C<'d, 'r>, TableId, TableId) {
    let limits = AssemblyLimits::default();
    let mut arena = Arena::new(limits);
    let root = arena.new_table().unwrap();
    let mut c = Context {
        arena,
        root,
        definitions: defs,
        request: r,
        dependencies: d,
        stage: "test",
        patterns: text::Patterns::new(limits),
        sequence: 0,
    };
    let p = &defs.policy().weapon;
    let base = c.fresh_field("base").unwrap();
    let weapon = c.arena.new_table().unwrap();
    c.arena
        .set_field(base, &p.base_field, Value::Table(weapon))
        .unwrap();
    c.arena
        .set_field(
            base,
            &p.type_base_field,
            Value::Text("finite weapon type".into()),
        )
        .unwrap();
    c.set(&p.name_item_field, Value::Text("finite weapon".into()))
        .unwrap();
    c.set("quality", Value::Number(20.0)).unwrap();
    c.fresh_field(&p.output_field).unwrap();
    for (key, value) in [
        (&p.attack_rate.base_field, 1.5),
        (&p.range.base_field, 10.0),
        (&p.critical.base_field, 5.0),
    ] {
        c.arena
            .set_field(weapon, key, Value::Number(value))
            .unwrap();
    }
    let list = c.mod_list().unwrap();
    (c, weapon, list)
}
fn add(c: &mut C<'_, '_>, list: TableId, q: &ItemAssemblyLocalQuery, v: Value) -> TableId {
    let row = c.arena.new_table().unwrap();
    c.arena
        .set_field(row, "name", Value::Text(q.name.clone()))
        .unwrap();
    c.arena
        .set_field(row, "type", Value::Text(q.mod_type.clone()))
        .unwrap();
    c.arena
        .set_field(row, "flags", Value::Number(q.flags))
        .unwrap();
    c.arena
        .set_field(row, "keywordFlags", Value::Number(0.0))
        .unwrap();
    c.arena.set_field(row, "value", v).unwrap();
    c.arena.append(list, Value::Table(row)).unwrap();
    row
}
fn row(c: &mut C<'_, '_>, list: TableId, name: &str, flags: f64) -> TableId {
    add(
        c,
        list,
        &ItemAssemblyLocalQuery {
            name: name.into(),
            mod_type: "BASE".into(),
            flags,
        },
        Value::Number(1.0),
    )
}
fn output(c: &mut C<'_, '_>, slot: i64) -> TableId {
    let slots = c
        .root_table(&c.definitions.policy().weapon.output_field)
        .unwrap();
    table(c.arena.get_index(slots, slot).unwrap()).unwrap()
}
fn n(c: &mut C<'_, '_>, id: TableId, key: &str) -> f64 {
    number(&c.arena.get_field(id, key).unwrap()).unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-10, "{a} != {b}");
}
fn override_value(c: &mut C<'_, '_>, list: TableId, key: Value, value: Value) -> TableId {
    let p = &c.definitions.policy().weapon.overrides;
    let payload = c.arena.new_table().unwrap();
    c.arena.set_field(payload, &p.key_field, key).unwrap();
    c.arena.set_field(payload, &p.value_field, value).unwrap();
    let q = ItemAssemblyLocalQuery {
        name: p.query_name.clone(),
        mod_type: c.definitions.policy().kinds.list.clone(),
        flags: 0.0,
    };
    add(c, list, &q, Value::Table(payload))
}

#[test]
fn ordered_weapon_formulas_cover_reload_elemental_and_unscaled_channels() {
    let r = request();
    let mut d = UnavailableItemLoadProvider;
    let defs = definitions();
    let (mut c, base, list) = setup(defs, &r, &mut d);
    let p = &defs.policy().weapon;
    c.arena
        .set_field(base, &p.reload.base_field, Value::Number(2.0))
        .unwrap();
    for (q, v) in [
        (&p.attack_speed.query, 20.0),
        (&p.attack_speed.alternate, 4.0),
        (&p.range.flat, 2.0),
        (&p.range.metres, 0.5),
        (&p.range.alternate, 1.0),
        (&p.reload.query, 20.0),
        (&p.damage.physical_increased, 20.0),
        (&p.damage.elemental_increased, 10.0),
        (&p.critical.base, 1.0),
        (&p.critical.increased, 10.0),
        (&p.critical.alternate, 2.0),
    ] {
        add(&mut c, list, q, Value::Number(v));
    }
    let physical = p
        .damage
        .channels
        .iter()
        .find(|x| x.kind == Kind::Physical)
        .unwrap();
    let elemental = p
        .damage
        .channels
        .iter()
        .find(|x| x.kind == Kind::Elemental)
        .unwrap();
    let unscaled = p
        .damage
        .channels
        .iter()
        .find(|x| x.kind == Kind::Unscaled)
        .unwrap();
    for (channel, min, max) in [
        (physical, 10.0, 20.0),
        (elemental, 2.0, 3.0),
        (unscaled, 2.25, 4.5),
    ] {
        c.arena
            .set_field(base, &channel.minimum.base_field, Value::Number(min))
            .unwrap();
        c.arena
            .set_field(base, &channel.maximum.base_field, Value::Number(max))
            .unwrap();
    }
    add(&mut c, list, &physical.minimum.query, Value::Number(2.0));
    add(&mut c, list, &physical.maximum.query, Value::Number(4.0));
    add(
        &mut c,
        list,
        elemental.increased.as_ref().unwrap(),
        Value::Number(20.0),
    );
    c.local_weapon(list, Some(1)).unwrap();
    let out = output(&mut c, 1);
    for (key, expected) in [
        (&p.attack_speed.output, 30.0),
        (&p.attack_rate.output, 1.95),
        (&p.range.bonus_output, 9.0),
        (&p.range.output, 19.0),
        (&p.reload.increased_output, 50.0),
        (&p.reload.output, 1.33),
        (&p.critical.output, 7.2),
        (&physical.minimum.output, 17.0),
        (&physical.maximum.output, 35.0),
        (&physical.dps_output, 50.7),
        (&elemental.minimum.output, 3.0),
        (&elemental.maximum.output, 4.0),
        (&p.damage.elemental_output, 6.825),
        (&unscaled.minimum.output, 2.25),
        (&unscaled.maximum.output, 4.5),
        (&p.total_output, 64.10625),
    ] {
        close(n(&mut c, out, key), expected);
    }
    for channel in
        p.damage.channels.iter().filter(|x| {
            x.name != physical.name && x.name != elemental.name && x.name != unscaled.name
        })
    {
        assert_eq!(
            c.arena.get_field(out, &channel.dps_output).unwrap(),
            Value::Nil
        );
    }
    assert_eq!(c.arena.dense_len(list).unwrap(), 0);
    let second = c.mod_list().unwrap();
    c.local_weapon(second, Some(2)).unwrap();
    assert_ne!(out, output(&mut c, 2));
    close(n(&mut c, out, &p.attack_rate.output), 1.95);
}

#[test]
fn overrides_alias_hand_tags_and_post_override_total_are_ordered() {
    let r = request();
    let mut d = UnavailableItemLoadProvider;
    let defs = definitions();
    let (mut c, _, list) = setup(defs, &r, &mut d);
    let p = &defs.policy().weapon;
    let first = &p.residual.untagged[0];
    let plain = row(&mut c, list, &first.names[0], first.flags.value);
    let critical = row(&mut c, list, &p.residual.critical.names[0], 0.0);
    let tag = c.arena.new_table().unwrap();
    c.arena
        .set_field(
            tag,
            "type",
            Value::Text(p.residual.condition_tag_type.clone()),
        )
        .unwrap();
    c.arena
        .set_field(
            tag,
            "var",
            Value::Text(p.residual.critical_condition.clone()),
        )
        .unwrap();
    c.arena.set_index(critical, 1, Value::Table(tag)).unwrap();
    c.arena
        .set_field(critical, "keywordFlags", Value::Number(12345.0))
        .unwrap(); // second branch has no keyword guard
    override_value(
        &mut c,
        list,
        Value::Text("alias".into()),
        Value::Table(plain),
    );
    override_value(
        &mut c,
        list,
        Value::Text(p.total_output.clone()),
        Value::Number(999.0),
    );
    let first_channel = &p.damage.channels[0];
    override_value(
        &mut c,
        list,
        Value::Text(first_channel.dps_output.clone()),
        Value::Text("12.5".into()),
    );
    c.local_weapon(list, Some(2)).unwrap();
    let out = output(&mut c, 2);
    assert_eq!(
        c.arena.get_field(out, "alias").unwrap(),
        Value::Table(plain)
    );
    let hand = table(c.arena.get_index(plain, 1).unwrap()).unwrap();
    assert_eq!(
        c.arena.get_field(hand, "var").unwrap(),
        Value::Text(p.residual.other_condition.clone())
    );
    assert_eq!(c.arena.get_index(critical, 1).unwrap(), Value::Table(tag));
    assert_eq!(c.arena.dense_len(critical).unwrap(), 2);
    assert_eq!(n(&mut c, out, &p.total_output), 12.5);
    let appended = table(c.arena.get_index(critical, 2).unwrap()).unwrap();
    assert_ne!(appended, hand);
}

#[test]
fn malformed_late_total_preserves_overrides_tags_and_reset() {
    let r = request();
    let mut d = UnavailableItemLoadProvider;
    let defs = definitions();
    let (mut c, _, list) = setup(defs, &r, &mut d);
    let p = &defs.policy().weapon;
    let rule = &p.residual.untagged[0];
    let residual = row(&mut c, list, &rule.names[0], rule.flags.value);
    override_value(
        &mut c,
        list,
        Value::Text(p.damage.channels[0].dps_output.clone()),
        Value::Text("not a number".into()),
    );
    assert_eq!(
        c.local_weapon(list, Some(1)).unwrap_err().kind,
        AssemblyErrorKind::Source
    );
    let out = output(&mut c, 1);
    assert_eq!(n(&mut c, out, &p.total_output), 0.0);
    assert!(c.arena.get_index(residual, 1).unwrap().as_table().is_some());
    assert_eq!(
        c.arena
            .get_field(out, &p.damage.channels[0].dps_output)
            .unwrap(),
        Value::Text("not a number".into())
    );
}

#[test]
fn early_quality_failure_and_nil_slot_keep_precise_publication_prefix() {
    let r = request();
    let mut d = UnavailableItemLoadProvider;
    let defs = definitions();
    let (mut c, _, list) = setup(defs, &r, &mut d);
    let p = &defs.policy().weapon;
    let first = add(&mut c, list, &p.attack_speed.query, Value::Number(5.0));
    let second = add(&mut c, list, &p.attack_speed.alternate, Value::Number(6.0));
    c.set("quality", Value::Boolean(false)).unwrap();
    assert_eq!(
        c.local_weapon(list, Some(1)).unwrap_err().kind,
        AssemblyErrorKind::Source
    );
    let out = output(&mut c, 1);
    assert_eq!(
        c.arena.get_field(out, &p.name_output).unwrap(),
        Value::Text("finite weapon".into())
    );
    assert_eq!(
        c.arena.get_field(out, &p.attack_speed.output).unwrap(),
        Value::Nil
    );
    assert_eq!(c.arena.dense_len(list).unwrap(), 1);
    assert_eq!(c.arena.get_index(list, 1).unwrap(), Value::Table(second));
    assert_ne!(first, second);
    assert_eq!(
        c.local_weapon(list, None).unwrap_err().kind,
        AssemblyErrorKind::Source
    );
    let slots = c.root_table(&p.output_field).unwrap();
    assert_eq!(c.arena.dense_len(slots).unwrap(), 1);
    assert_eq!(c.arena.dense_len(list).unwrap(), 1);
}

#[test]
fn nonfinite_reload_keeps_prior_speed_range_and_reload_increase() {
    let r = request();
    let mut d = UnavailableItemLoadProvider;
    let defs = definitions();
    let (mut c, base, list) = setup(defs, &r, &mut d);
    let p = &defs.policy().weapon;
    c.arena
        .set_field(base, &p.reload.base_field, Value::Number(2.0))
        .unwrap();
    add(&mut c, list, &p.reload.query, Value::Number(-100.0));
    assert_eq!(
        c.local_weapon(list, Some(1)).unwrap_err().kind,
        AssemblyErrorKind::Unsupported
    );
    let out = output(&mut c, 1);
    assert_eq!(n(&mut c, out, &p.attack_rate.output), 1.5);
    assert_eq!(n(&mut c, out, &p.reload.increased_output), -100.0);
    assert_eq!(
        c.arena.get_field(out, &p.reload.output).unwrap(),
        Value::Nil
    );
    assert_eq!(
        c.arena.get_field(out, &p.critical.output).unwrap(),
        Value::Nil
    );
}

#[test]
fn changed_channel_order_names_outputs_and_queries_are_consumed() {
    let mut raw = data().item_assembly().data().clone();
    let p = &mut raw.policy.weapon;
    p.base_field = "modelWeapon".into();
    p.output_field = "modelWeaponData".into();
    p.damage.channels.reverse();
    p.attack_speed.query.name = "modelSpeed".into();
    p.attack_rate.output = "modelRate".into();
    p.percent_divisor = 50.0;
    let channel = &mut p.damage.channels[0];
    channel.name = "model channel".into();
    channel.kind = Kind::Unscaled;
    channel.increased = None;
    channel.minimum.base_field = "modelMin".into();
    channel.maximum.base_field = "modelMax".into();
    channel.minimum.output = "modelMinOut".into();
    channel.maximum.output = "modelMaxOut".into();
    channel.dps_output = "modelDps".into();
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
    let (mut c, base, list) = setup(defs, &r, &mut d);
    let p = &defs.policy().weapon;
    add(&mut c, list, &p.attack_speed.query, Value::Number(50.0));
    c.arena
        .set_field(base, "modelMin", Value::Number(2.5))
        .unwrap();
    c.arena
        .set_field(base, "modelMax", Value::Number(3.5))
        .unwrap();
    c.local_weapon(list, Some(1)).unwrap();
    let out = output(&mut c, 1);
    assert_eq!(n(&mut c, out, "modelRate"), 3.0);
    assert_eq!(n(&mut c, out, "modelDps"), 9.0);
    assert_eq!(n(&mut c, out, &p.total_output), 9.0);
    assert_eq!(c.get("weaponData").unwrap(), Value::Nil);
}

#[test]
fn ordinary_native_provider_finishes_two_slot_melee_and_reload_weapons() {
    let d = data();
    let p = &d.item_assembly().policy().weapon;
    let reload=d.item_loading().bases().iter().find(|base| {
        matches!(base.fields.fields.get(&p.base_field),Some(ItemMetadataValue::Table(fields)) if fields.fields.contains_key(&p.reload.base_field))
    }).expect("original reload weapon definition");
    for name in ["Wooden Club", reload.name.as_str()] {
        let mut machine = ItemLoadMachine::new(d.item_loading());
        machine.set_xml_attributes(&[("id".into(), "1".into())].into());
        let mut provider = BuiltinItemLoadProvider::new(d);
        machine
            .apply_text(
                &format!("Rarity: Normal\n{name}\nQuality: +0%\nImplicits: 0"),
                &mut provider,
            )
            .unwrap();
        machine.finish_load(&mut provider).unwrap();
        let item = machine
            .assembled()
            .expect("normal native weapon producer must finish");
        let weapons = item
            .field(item.root(), &p.output_field)
            .unwrap()
            .as_table()
            .unwrap();
        let slots = item
            .field(item.root(), "slotModList")
            .unwrap()
            .as_table()
            .unwrap();
        let a = item.index(weapons, 1).unwrap().as_table().unwrap();
        let b = item.index(weapons, 2).unwrap().as_table().unwrap();
        assert_ne!(a, b);
        for index in [1, 2] {
            let weapon = item.index(weapons, index).unwrap().as_table().unwrap();
            assert!(
                item.field(weapon, &p.attack_rate.output)
                    .unwrap()
                    .as_number()
                    .is_some()
            );
            assert!(
                item.field(weapon, &p.total_output)
                    .unwrap()
                    .as_number()
                    .is_some()
            );
            assert!(item.index(slots, index).unwrap().as_table().is_some());
            if name == reload.name {
                assert!(
                    item.field(weapon, &p.reload.output)
                        .unwrap()
                        .as_number()
                        .is_some()
                );
            }
        }
    }
}
