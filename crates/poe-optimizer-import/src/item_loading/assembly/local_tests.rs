use super::*;
use crate::item_loading::{ArmourDataUpdate, ItemNumber};
use poe_optimizer_data::item_assembly::{ItemAssemblyArmourRole as Role, ItemAssemblyLocalQuery};

type C<'d, 'r> = Context<'d, 'r, UnavailableItemLoadProvider>;
fn initialize(
    c: &mut C<'_, '_>,
    base_field: &str,
    output_field: &str,
) -> (TableId, TableId, TableId) {
    let root_base = c.fresh_field("base").unwrap();
    let base = c.arena.new_table().unwrap();
    c.arena
        .set_field(root_base, base_field, Value::Table(base))
        .unwrap();
    let output = c.fresh_field(output_field).unwrap();
    c.set("quality", Value::Number(25.0)).unwrap();
    c.set("modSource", Value::Text("finite-test".into()))
        .unwrap();
    let list = c.mod_list().unwrap();
    (base, output, list)
}
fn add(c: &mut C<'_, '_>, list: TableId, query: &ItemAssemblyLocalQuery, value: Value) -> TableId {
    let row = c
        .arena
        .import_metadata(&record(
            &query.name,
            &query.mod_type,
            ItemMetadataValue::Number(0.0),
        ))
        .unwrap();
    c.arena
        .set_field(row, "flags", Value::Number(query.flags))
        .unwrap();
    c.arena.set_field(row, "value", value).unwrap();
    c.arena.append(list, Value::Table(row)).unwrap();
    row
}
fn n(c: &mut C<'_, '_>, table: TableId, key: &str) -> f64 {
    number(&c.arena.get_field(table, key).unwrap()).unwrap()
}
fn armour_query(c: &C<'_, '_>, role: Role) -> ItemAssemblyLocalQuery {
    c.definitions
        .policy()
        .armour
        .queries
        .iter()
        .find(|q| q.role == role)
        .unwrap()
        .query
        .clone()
}
fn override_row(c: &mut C<'_, '_>, list: TableId, name: &str, key: Value, value: Value) -> TableId {
    let payload = c.arena.new_table().unwrap();
    c.arena.set_field(payload, "key", key).unwrap();
    c.arena.set_field(payload, "value", value).unwrap();
    let query = ItemAssemblyLocalQuery {
        name: name.into(),
        mod_type: c.definitions.policy().kinds.list.clone(),
        flags: 0.0,
    };
    add(c, list, &query, Value::Table(payload));
    payload
}

#[test]
fn armour_formula_removals_and_conditional_movement_residual() {
    let request = request();
    let mut d = UnavailableItemLoadProvider;
    let mut c = context(&request, &mut d);
    let p = c.definitions.policy().armour.clone();
    let (base, out, list) = initialize(&mut c, &p.base_field, &p.output_field);
    for (rule, value) in p.defences.iter().zip([100.0, 80.0, 60.0, 20.0]) {
        c.arena
            .set_field(base, &rule.base.field, Value::Number(value))
            .unwrap();
    }
    for (role, value) in [
        (Role::ArmourBase, 10.0),
        (Role::ArmourEvasionBase, 5.0),
        (Role::ArmourIncreased, 20.0),
        (Role::DefencesIncreased, 10.0),
        (Role::EvasionPerLevel, 2.0),
    ] {
        let q = armour_query(&c, role);
        add(&mut c, list, &q, Value::Number(value));
    }
    c.arena
        .set_field(base, &p.block.base_field, Value::Number(25.0))
        .unwrap();
    c.arena
        .set_field(base, &p.movement.base_field, Value::Number(5.0))
        .unwrap();
    add(&mut c, list, &p.block.base, Value::Number(3.0));
    add(&mut c, list, &p.block.increased, Value::Number(10.0));
    c.local_armour(list).unwrap();
    for (rule, value) in p.defences.iter().zip([187.0, 117.0, 83.0, 28.0]) {
        assert_eq!(n(&mut c, out, &rule.output), value);
    }
    assert_eq!(n(&mut c, out, &p.per_level[0].output), 2.75);
    assert_eq!(n(&mut c, out, &p.block.output), 30.0);
    assert_eq!(c.arena.dense_len(list).unwrap(), 1);
    let residual = table(c.arena.get_index(list, 1).unwrap()).unwrap();
    assert_eq!(n(&mut c, residual, "value"), -5.0);
    let tag = table(c.arena.get_index(residual, 1).unwrap()).unwrap();
    assert_eq!(
        c.arena.get_field(tag, "var").unwrap(),
        Value::Text(p.movement.condition)
    );
    assert_eq!(c.arena.get_field(tag, "neg").unwrap(), Value::Boolean(true));
}

#[test]
fn armour_error_follows_query_removal_and_first_base_write() {
    let request = request();
    let mut d = UnavailableItemLoadProvider;
    let mut c = context(&request, &mut d);
    let p = c.definitions.policy().armour.clone();
    let (_, out, list) = initialize(&mut c, &p.base_field, &p.output_field);
    let q = armour_query(&c, Role::ArmourBase);
    add(&mut c, list, &q, Value::Number(9.0));
    c.set("quality", Value::Boolean(false)).unwrap();
    assert_eq!(
        c.local_armour(list).unwrap_err().kind,
        AssemblyErrorKind::Source
    );
    assert_eq!(c.arena.dense_len(list).unwrap(), 0);
    assert_eq!(n(&mut c, out, &p.defences[0].base_output), 0.0);
    assert_eq!(
        c.arena.get_field(out, &p.defences[0].output).unwrap(),
        Value::Nil
    );
    // The source alternate-quality query can suppress the same invalid value.
    add(&mut c, list, &p.alternate_quality, Value::Number(1.0));
    c.local_armour(list).unwrap();
    assert_eq!(n(&mut c, out, &p.defences[0].output), 0.0);
}

#[test]
fn late_overrides_keep_alias_deletion_and_error_prefix() {
    let request = request();
    let mut d = UnavailableItemLoadProvider;
    let mut c = context(&request, &mut d);
    let p = c.definitions.policy().armour.clone();
    let (_, out, list) = initialize(&mut c, &p.base_field, &p.output_field);
    let child = c.arena.new_table().unwrap();
    override_row(
        &mut c,
        list,
        &p.overrides.query_name,
        Value::Text("custom".into()),
        Value::Table(child),
    );
    override_row(
        &mut c,
        list,
        &p.overrides.query_name,
        Value::Text(p.defences[0].output.clone()),
        Value::Nil,
    );
    override_row(
        &mut c,
        list,
        &p.overrides.query_name,
        Value::Nil,
        Value::Number(9.0),
    );
    assert_eq!(
        c.local_armour(list).unwrap_err().kind,
        AssemblyErrorKind::Source
    );
    assert_eq!(
        c.arena.get_field(out, "custom").unwrap(),
        Value::Table(child)
    );
    assert_eq!(
        c.arena.get_field(out, &p.defences[0].output).unwrap(),
        Value::Nil
    );
    assert_eq!(c.arena.dense_len(list).unwrap(), 3); // List query is non-destructive.
    c.arena
        .set_field(child, "changed", Value::Boolean(false))
        .unwrap();
    let alias = table(c.arena.get_field(out, "custom").unwrap()).unwrap();
    assert_eq!(
        c.arena.get_field(alias, "changed").unwrap(),
        Value::Boolean(false)
    );
}

#[test]
fn flask_recovery_consumes_flags_from_base_and_keeps_fractional_capacity() {
    let request = request();
    let mut d = UnavailableItemLoadProvider;
    let mut c = context(&request, &mut d);
    let p = c.definitions.policy().flask.clone();
    let (base, out, list) = initialize(&mut c, &p.base_field, &p.output_field);
    let original = c.mod_list().unwrap();
    c.arena
        .set_field(base, &p.duration.base_field, Value::Number(6.0))
        .unwrap();
    c.arena
        .set_field(base, &p.charges.maximum_base_field, Value::Number(31.0))
        .unwrap();
    c.arena
        .set_field(base, &p.charges.used_base_field, Value::Number(11.0))
        .unwrap();
    for channel in &p.recovery.channels {
        c.arena
            .set_field(base, &channel.base_field, Value::Number(80.0))
            .unwrap();
        let row = add(
            &mut c,
            original,
            &channel.effect_not_removed.query,
            Value::Boolean(true),
        );
        c.arena.append(list, Value::Table(row)).unwrap();
    }
    add(&mut c, list, &p.recovery.instant.query, Value::Number(25.0));
    add(&mut c, list, &p.recovery.increased, Value::Number(20.0));
    add(&mut c, list, &p.recovery.rate, Value::Number(50.0));
    add(&mut c, list, &p.duration.increased, Value::Number(50.0));
    add(
        &mut c,
        list,
        &p.charges.maximum_increased,
        Value::Number(10.0),
    );
    add(
        &mut c,
        list,
        &p.charges.used_increased,
        Value::Number(-10.0),
    );
    c.local_flask(list, original).unwrap();
    assert_eq!(n(&mut c, out, &p.duration.output), 6.0);
    assert_eq!(n(&mut c, out, &p.charges.maximum_output), 31.0 * 1.1);
    assert_eq!(n(&mut c, out, &p.charges.used_output), 9.0);
    for channel in &p.recovery.channels {
        assert_eq!(n(&mut c, out, &channel.base_output), 120.0);
        assert_eq!(n(&mut c, out, &channel.instant_output), 30.0);
        assert_eq!(n(&mut c, out, &channel.gradual_output), 90.0);
        assert_eq!(n(&mut c, out, &channel.total_output), 120.0);
        assert_eq!(
            c.arena
                .get_field(out, &channel.effect_not_removed.output)
                .unwrap(),
            Value::Boolean(true)
        );
    }
    assert_eq!(c.arena.dense_len(original).unwrap(), 0);
    assert_eq!(c.arena.dense_len(list).unwrap(), 2);
}

#[test]
fn charm_quality_duration_and_failure_keep_reached_writes() {
    let request = request();
    let mut d = UnavailableItemLoadProvider;
    let mut c = context(&request, &mut d);
    let p = c.definitions.policy().charm.clone();
    let (base, out, list) = initialize(&mut c, &p.base_field, &p.output_field);
    c.arena
        .set_field(base, &p.duration.base_field, Value::Number(4.0))
        .unwrap();
    c.arena
        .set_field(base, &p.charges.maximum_base_field, Value::Number(10.0))
        .unwrap();
    c.arena
        .set_field(base, &p.charges.used_base_field, Value::Boolean(false))
        .unwrap();
    add(&mut c, list, &p.duration.increased, Value::Number(20.0));
    add(&mut c, list, &p.duration.more, Value::Number(50.0));
    add(&mut c, list, &p.charges.used_increased, Value::Number(10.0));
    assert_eq!(
        c.local_charm(list).unwrap_err().kind,
        AssemblyErrorKind::Source
    );
    assert_eq!(n(&mut c, out, &p.duration.output), 9.0);
    assert_eq!(n(&mut c, out, &p.charges.maximum_output), 10.0);
    assert_eq!(
        c.arena.get_field(out, &p.charges.used_output).unwrap(),
        Value::Nil
    );
    assert_eq!(c.arena.dense_len(list).unwrap(), 0); // query precedes bad base arithmetic.
}

#[test]
fn zero_recovery_rate_has_explicit_nonfinite_prefix() {
    let request = request();
    let mut d = UnavailableItemLoadProvider;
    let mut c = context(&request, &mut d);
    let p = c.definitions.policy().flask.clone();
    let (base, out, list) = initialize(&mut c, &p.base_field, &p.output_field);
    let original = c.mod_list().unwrap();
    c.arena
        .set_field(base, &p.duration.base_field, Value::Number(4.0))
        .unwrap();
    c.arena
        .set_field(
            base,
            &p.recovery.channels[0].base_field,
            Value::Number(10.0),
        )
        .unwrap();
    add(&mut c, list, &p.recovery.instant.query, Value::Number(30.0));
    add(&mut c, list, &p.recovery.rate, Value::Number(-100.0));
    assert_eq!(
        c.local_flask(list, original).unwrap_err().kind,
        AssemblyErrorKind::Unsupported
    );
    assert_eq!(n(&mut c, out, &p.recovery.instant.output), 30.0);
    assert_eq!(
        c.arena.get_field(out, &p.duration.output).unwrap(),
        Value::Nil
    );
    assert_eq!(c.arena.dense_len(list).unwrap(), 0);
}

#[test]
fn incomplete_numeric_shadow_requires_owned_graph_and_is_not_replayed() {
    let r = request();
    let mut incomplete = r.clone();
    incomplete.state.armour_data_complete = false;
    incomplete.state.armour_data = Some([("custom".into(), ItemNumber::new(999.0))].into());
    let mut d = UnavailableItemLoadProvider;
    let mut c = context(&incomplete, &mut d);
    assert_eq!(
        c.hydrate(false).unwrap_err().kind,
        AssemblyErrorKind::Unsupported
    );
    c.request = &r;
    c.hydrate(false).unwrap();
    let armour = c.fresh_field("armourData").unwrap();
    let child = c.arena.new_table().unwrap();
    c.arena
        .set_field(armour, "custom", Value::Table(child))
        .unwrap();
    c.request = &incomplete;
    c.hydrate(true).unwrap();
    assert_eq!(c.get("armourData").unwrap(), Value::Table(armour));
    assert_eq!(
        c.arena.get_field(armour, "custom").unwrap(),
        Value::Table(child)
    );
    let item = c.arena.finish(c.root).unwrap();
    let projected = loading_updates(&item, &r.state).unwrap();
    assert!(
        matches!(projected.armour_data,ArmourDataUpdate::NumericSubset(ref fields) if fields.is_empty())
    );
    assert!(projected.assembled.unwrap().shares_storage_with(&item));
}

#[test]
fn provider_keeps_nested_armour_graph_through_final_and_header_reparse() {
    use crate::item_loading::NativeItemLoadProvider;
    struct Parser {
        text: String,
        rows: Vec<ItemMetadataTable>,
    }
    impl ItemLoadProvider for Parser {
        fn parse_modifier(&mut self, r: &ParseRequest) -> DependencyResult<ParseOutcome> {
            DependencyResult::Available(ParseOutcome {
                modifiers: Some(if r.text == self.text {
                    self.rows.clone()
                } else {
                    Vec::new()
                }),
                extra: None,
            })
        }
    }
    let p = &definitions().policy().armour;
    let custom = ItemMetadataValue::Table(ItemMetadataTable {
        fields: [("inside".into(), ItemMetadataValue::Boolean(false))].into(),
        indexed: BTreeMap::new(),
    });
    let payload = |key: &str, value: ItemMetadataValue| {
        record(
            &p.overrides.query_name,
            "LIST",
            ItemMetadataValue::Table(ItemMetadataTable {
                fields: [
                    (
                        p.overrides.key_field.clone(),
                        ItemMetadataValue::Text(key.into()),
                    ),
                    (p.overrides.value_field.clone(), value),
                ]
                .into(),
                indexed: BTreeMap::new(),
            }),
        )
    };
    let probe = "finite nested armour override";
    let parser = Parser {
        text: probe.into(),
        rows: vec![
            payload("custom", custom),
            payload(&p.block.output, ItemMetadataValue::Number(33.5)),
        ],
    };
    let mut provider = NativeItemLoadProvider::with_native_assembly(data(), parser);
    let mut machine = ItemLoadMachine::new(data().item_loading());
    machine.set_xml_attributes(&[("id".into(), "1".into())].into());
    machine
        .apply_text(
            &format!("Rarity: Normal\nRusted Greathelm\nQuality: +0%\nImplicits: 0\n{probe}"),
            &mut provider,
        )
        .unwrap();
    assert!(!machine.state().armour_data_complete);
    assert!(machine.assembly_progress().unwrap().is_complete());
    assert!(machine.assembled().is_none());
    machine.finish_load(&mut provider).unwrap();
    let prior = machine
        .assembled()
        .expect("native provider must admit authoritative graph")
        .clone();
    let armour = prior
        .field(prior.root(), "armourData")
        .unwrap()
        .as_table()
        .unwrap();
    let nested = prior.field(armour, "custom").unwrap().as_table().unwrap();
    assert_eq!(prior.field(nested, "inside"), Some(&Value::Boolean(false)));
    assert_eq!(
        prior.field(armour, &p.block.output),
        Some(&Value::Number(33.5))
    );
    assert!(!machine.state().armour_data_complete);
    assert!(
        !machine
            .state()
            .armour_data
            .as_ref()
            .unwrap()
            .contains_key("custom")
    );
    // ParseRaw can update persistent armour headers without finding a base or
    // invoking assembly. Those writes must remain pending until a real assembly.
    machine
        .apply_text(
            "Rarity: Normal\nUnknown finite local fixture\nArmour: 777",
            &mut provider,
        )
        .unwrap();
    assert_eq!(
        machine.status(),
        crate::item_loading::ItemLoadStatus::NoBase
    );
    machine
        .apply_text("Rarity: Normal\nGold Ring\nImplicits: 0", &mut provider)
        .unwrap();
    let recovered = machine.assembly_progress().unwrap();
    assert!(recovered.is_complete());
    assert_eq!(
        recovered.field(recovered.root(), "armourData"),
        Some(&Value::Table(armour))
    );
    assert_eq!(
        recovered.field(armour, "Armour"),
        Some(&Value::Number(777.0))
    );
    assert_eq!(
        recovered.field(armour, "custom"),
        Some(&Value::Table(nested))
    );
    machine.finish_load(&mut provider).unwrap();
    machine.apply_text("Rarity: Normal\nGold Ring\nArmour: 999\nEvasion: 23\nChance to Block: 47%\nImplicits: 0",&mut provider).unwrap();
    let reparsed = machine.assembly_progress().unwrap();
    assert!(reparsed.is_complete());
    assert_eq!(
        reparsed.field(reparsed.root(), "armourData"),
        Some(&Value::Table(armour))
    );
    assert_eq!(
        reparsed.field(armour, "custom"),
        Some(&Value::Table(nested))
    );
    assert_eq!(
        reparsed.field(armour, "Armour"),
        Some(&Value::Number(999.0))
    );
    assert_eq!(
        reparsed.field(armour, "Evasion"),
        Some(&Value::Number(23.0))
    );
    assert_eq!(
        reparsed.field(armour, &p.block.output),
        Some(&Value::Number(33.5))
    ); // ignored display header
    machine.finish_load(&mut provider).unwrap();
    let final_item = machine.assembled().unwrap();
    assert_eq!(
        final_item.field(armour, "custom"),
        Some(&Value::Table(nested))
    );
    assert_eq!(
        final_item.field(armour, "Armour"),
        Some(&Value::Number(999.0))
    );
    assert!(!machine.state().armour_data_complete);
}

#[test]
fn custom_query_values_and_bad_family_base_fail_at_original_use() {
    use poe_optimizer_data::item_assembly::ItemAssemblyCatalog;
    let mut custom = data().item_assembly().data().clone();
    let q = custom
        .policy
        .armour
        .queries
        .iter_mut()
        .find(|q| q.role == Role::ArmourEvasionBase)
        .unwrap();
    q.query.mod_type = custom.policy.kinds.flag.clone();
    custom.policy.flask.duration.increased.mod_type = custom.policy.kinds.flag.clone();
    let owner = ItemAssemblyCatalog::new(custom).unwrap();
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
    let initial = context(&r, &mut d);
    let mut c = Context {
        definitions: defs,
        ..initial
    };
    let p = c.definitions.policy().armour.clone();
    let (_, out, list) = initialize(&mut c, &p.base_field, &p.output_field);
    let bad = armour_query(&c, Role::ArmourEvasionBase);
    let later = armour_query(&c, Role::DefencesIncreased);
    add(&mut c, list, &bad, Value::Boolean(false));
    add(&mut c, list, &later, Value::Number(10.0));
    add(&mut c, list, &p.alternate_quality, Value::Number(0.0));
    assert_eq!(
        c.local_armour(list).unwrap_err().kind,
        AssemblyErrorKind::Source
    );
    assert_eq!(c.arena.dense_len(list).unwrap(), 0);
    assert_eq!(n(&mut c, out, &p.defences[0].base_output), 0.0);
    assert_eq!(
        c.arena.get_field(out, &p.defences[0].output).unwrap(),
        Value::Nil
    );
    let f = c.definitions.policy().flask.clone();
    let (base, out, list) = initialize(&mut c, &f.base_field, &f.output_field);
    c.arena
        .set_field(base, &f.charges.maximum_base_field, Value::Number(10.0))
        .unwrap();
    c.arena
        .set_field(base, &f.charges.used_base_field, Value::Number(2.0))
        .unwrap();
    add(&mut c, list, &f.duration.increased, Value::Boolean(false));
    add(&mut c, list, &f.duration.more, Value::Number(20.0));
    let original = c.mod_list().unwrap();
    // No recovery channel means these bare duration locals are never arithmetic.
    c.local_flask(list, original).unwrap();
    assert_eq!(
        c.arena.get_field(out, &f.duration.output).unwrap(),
        Value::Nil
    );
    let root_base = c.root_table("base").unwrap();
    c.arena
        .set_field(root_base, &f.base_field, Value::Boolean(true))
        .unwrap();
    add(&mut c, list, &f.duration.increased, Value::Boolean(false));
    add(&mut c, list, &f.duration.more, Value::Number(20.0));
    assert_eq!(
        c.local_flask(list, original).unwrap_err().kind,
        AssemblyErrorKind::Source
    );
    assert_eq!(c.arena.dense_len(list).unwrap(), 0);
}
