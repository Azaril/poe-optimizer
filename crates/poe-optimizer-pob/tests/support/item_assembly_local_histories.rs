//! Local-family cases use full original methods. Derived modifiers are declared
//! finite entry inputs, separately labelled from actual parser-driven histories.
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
fn numeric(name: &str, kind: &str, n: f64) -> Metadata {
    modifier(name, kind, Field::Number(n))
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
fn raw(base: &str, quality: i32, properties: &str) -> String {
    format!(
        "Rarity: Normal\n{base}\nQuality: +{quality}%\n{properties}Implicits: 0\n+10 to maximum Life"
    )
}

#[derive(Clone, Copy)]
enum Expected {
    Complete,
    SourceError,
    NonFiniteFrontier,
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
    fn derived(
        &self,
        label: &str,
        base: &str,
        modifiers: Vec<Metadata>,
        expected: Expected,
    ) -> Json {
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
            &raw(base, 17, ""),
            &format!("{label} entry preparation"),
        );
        let mut request = dependency.requests.first().unwrap().clone();
        assert!(request.reparsed && request.previous.is_none());
        assert_eq!(request.state.explicit_mod_lines.len(), 1);
        let before: Table = event
            .raw_get::<Table>("before")
            .unwrap()
            .raw_get("root")
            .unwrap();
        before.set_metatable(Some(self.class.clone())).unwrap();
        let lines: Table = before.raw_get("explicitModLines").unwrap();
        assert_eq!(lines.raw_len(), 1);
        let row: Table = lines.raw_get(1).unwrap();
        assert!(matches!(row.raw_get::<Value>("extra").unwrap(), Value::Nil));
        assert!(request.state.explicit_mod_lines[0].extra.is_none());
        let source_inputs = pack(self.lua, &modifiers);
        request.state.explicit_mod_lines[0].modifiers = modifiers.clone();
        row.raw_set("modList", source_inputs.clone()).unwrap();
        // Independently materialize the exact native request values and compare
        // the complete ordered input graph before either assembler is called.
        let input_graph = graph(source_inputs);
        assert_eq!(
            input_graph,
            graph(pack(
                self.lua,
                &request.state.explicit_mod_lines[0].modifiers
            ))
        );
        let input_sha = super::super::hash(&serde_json::to_vec(&input_graph).unwrap());
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
                json!({"status":"complete", "graph":compare_graph(&result, &event, label)})
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
                let prefix = attempt.partial.expect("source failure must retain prefix");
                assert!(!prefix.is_complete());
                json!({"status":"source_error", "source_error":source_error,
                    "native_error":native_error.message,"stage":attempt.stage,
                    "prefix":compare_graph(&prefix,&event,label)})
            }
            Expected::NonFiniteFrontier => {
                source_result.unwrap();
                assert!(event.raw_get::<bool>("completed").unwrap());
                let duration: f64 = before
                    .raw_get::<Table>("flaskData")
                    .unwrap()
                    .raw_get("duration")
                    .unwrap();
                assert!(
                    !duration.is_finite(),
                    "zero rate must exercise nonfinite source output"
                );
                let native_error = attempt.result.unwrap_err();
                assert_eq!(native_error.kind, assembly::AssemblyErrorKind::Unsupported);
                assert!(attempt.partial.as_ref().is_some_and(|p| !p.is_complete()));
                json!({"status":"explicit_nonfinite_frontier","source_completed":true,
                    "source_duration_bits":format!("{:016x}",duration.to_bits()),
                    "native_error":native_error.message,"stage":attempt.stage,
                    "final_graph_parity_claimed":false})
            }
        };
        json!({"label":label,"base":base,"scope":"derived finite pre-call modifier input; not a parser-return claim",
            "input_modifiers":modifiers,"ordered_input_graph_sha256":input_sha,
            "same_ordered_native_source_inputs_verified":true,"source_poststate_used_as_dependency":false,
            "registration_authorized":false,"outcome":outcome})
    }
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
    let cx = Context {
        lua,
        module,
        class,
        parse,
        build,
        parser,
        snapshot,
    };
    let mut fresh = Vec::new();
    for (base, quality) in [
        ("Rusted Cuirass", 17),
        ("Painted Tower Shield", 21),
        ("Iron Greaves", 0),
        ("Ultimate Life Flask", 17),
        ("Ultimate Mana Flask", 23),
        ("Ruby Charm", 11),
    ] {
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
            &raw(base, quality, ""),
            base,
        );
        let finished = finish_step(lua, module, &source, build, &mut native, &mut p, base);
        fresh.push(json!({"base":base,"quality":quality,"parse":parsed,"final":finished}));
    }
    let source = item(lua);
    let mut native = machine(snapshot);
    let mut p = provider(snapshot, parser);
    let mut reparse = Vec::new();
    let mut retained_armour: Option<Table> = None;
    for (base, quality, properties) in [
        ("Painted Tower Shield", 17, ""),
        (
            "Gold Ring",
            0,
            "Armour: 777\nEvasion: 23\nChance to Block: 47%\n",
        ),
        ("Rusted Cuirass", 11, ""),
        ("Ultimate Life Flask", 19, ""),
        ("Ruby Charm", 13, ""),
        ("Gold Ring", 0, ""),
        ("Painted Tower Shield", 7, ""),
    ] {
        let (parsed, _) = parse_step(
            lua,
            module,
            &source,
            parse,
            &mut native,
            &mut p,
            &raw(base, quality, properties),
            base,
        );
        let armour: Table = source.raw_get("armourData").unwrap();
        if let Some(prior) = &retained_armour {
            assert_eq!(
                prior, &armour,
                "original armourData identity must survive reparse"
            );
        } else {
            retained_armour = Some(armour);
        }
        let finished = finish_step(lua, module, &source, build, &mut native, &mut p, base);
        reparse.push(json!({"base":base,"quality":quality,"properties":properties,"parse":parsed,"final":finished,
            "original_armour_data_identity_retained":true}));
    }
    // A no-base ParseRaw still writes display headers into the persistent
    // armour table. Those writes are not committed by the early BuildModList
    // return and must survive until the next valid native assembly.
    let source = item(lua);
    let mut native = machine(snapshot);
    let mut p = provider(snapshot, parser);
    let (shield_parse, _) = parse_step(
        lua,
        module,
        &source,
        parse,
        &mut native,
        &mut p,
        &raw("Painted Tower Shield", 17, ""),
        "no-base history shield parse",
    );
    let shield_final = finish_step(
        lua,
        module,
        &source,
        build,
        &mut native,
        &mut p,
        "no-base history shield final",
    );
    let armour: Table = source.raw_get("armourData").unwrap();
    let absent = "Rarity: Normal\nNo matching R2z base\nArmour: 777";
    let (source_result, no_base_report) =
        observe(lua, module, || parse.call::<()>((source.clone(), absent)));
    source_result.unwrap();
    native.apply_text(absent, &mut p).unwrap();
    assert!(matches!(
        source.raw_get::<Value>("base").unwrap(),
        Value::Nil
    ));
    assert_eq!(
        native.status(),
        poe_optimizer_import::item_loading::ItemLoadStatus::NoBase
    );
    assert!(!native.state().base_present);
    assert!(
        native.assembled().is_none(),
        "NoBase cannot expose a final assembly"
    );
    assert_eq!(source.raw_get::<Table>("armourData").unwrap(), armour);
    assert_eq!(armour.raw_get::<f64>("Armour").unwrap(), 777.0);
    assert_eq!(
        native.state().armour_data.as_ref().unwrap()["Armour"].value(),
        Some(777.0)
    );
    assert!(last(&no_base_report).raw_get::<bool>("completed").unwrap());
    let (ring_parse, _) = parse_step(
        lua,
        module,
        &source,
        parse,
        &mut native,
        &mut p,
        &raw("Gold Ring", 0, ""),
        "no-base history following ring parse",
    );
    let ring_final = finish_step(
        lua,
        module,
        &source,
        build,
        &mut native,
        &mut p,
        "no-base history following ring final",
    );
    assert_eq!(source.raw_get::<Table>("armourData").unwrap(), armour);
    assert_eq!(armour.raw_get::<f64>("Armour").unwrap(), 777.0);
    let no_base_reparse = json!({"shield_parse":shield_parse,"shield_final":shield_final,
        "no_base":{"raw":absent,"source_no_base":true,"native_status":"NoBase",
            "header_armour":777,"native_assembled_available":false,
            "source_armour_identity_retained":true,"assembled_graph_parity_claimed":false},
        "following_ring_parse":ring_parse,"following_ring_final":ring_final});
    let mut derived = Vec::new();
    let mut armour = Vec::new();
    for (name, n) in [
        ("Armour", 13.25),
        ("Evasion", 7.5),
        ("EnergyShield", 9.75),
        ("Ward", 2.5),
        ("ArmourAndEvasion", 3.25),
        ("ArmourAndEnergyShield", 4.5),
        ("EvasionAndEnergyShield", 1.75),
        ("EvasionPerLevel", 1.25),
        ("EnergyShieldPerLevel", 2.75),
        ("WardPerLevel", 0.5),
        ("BlockChance", 1.5),
    ] {
        armour.push(numeric(name, "BASE", n));
    }
    for name in [
        "Armour",
        "Evasion",
        "EnergyShield",
        "Ward",
        "ArmourAndEvasion",
        "ArmourAndEnergyShield",
        "EvasionAndEnergyShield",
        "Defences",
        "BlockChance",
    ] {
        armour.push(numeric(name, "INC", 12.5));
    }
    derived.push(cx.derived(
        "armour defence block movement",
        "Painted Tower Shield",
        armour.clone(),
        Expected::Complete,
    ));
    armour.push(numeric("AlternateQualityArmour", "BASE", 1.0));
    derived.push(cx.derived(
        "armour alternate quality suppression",
        "Painted Tower Shield",
        armour,
        Expected::Complete,
    ));
    let nested = Field::Table(record([
        ("label", Field::Text("retained override".into())),
        (
            "values",
            Field::Array(vec![Field::Boolean(false), Field::Number(2.5)]),
        ),
    ]));
    for (base, list, key) in [
        ("Rusted Cuirass", "ArmourData", "Armour"),
        ("Ultimate Life Flask", "FlaskData", "chargesUsed"),
        ("Ruby Charm", "CharmData", "duration"),
    ] {
        derived.push(cx.derived(
            &format!("{list} ordered overwrite nil nested alias"),
            base,
            vec![
                override_value(list, Some(key), Some(Field::Number(13.5))),
                override_value(list, Some(key), Some(Field::Number(27.25))),
                override_value(list, Some(key), None),
                override_value(list, Some("custom"), Some(nested.clone())),
            ],
            Expected::Complete,
        ));
    }
    let mut recovery = vec![
        numeric("Duration", "INC", 13.5),
        numeric("Duration", "MORE", 17.5),
        numeric("FlaskInstantRecovery", "BASE", 37.5),
        numeric("FlaskRecovery", "INC", 21.25),
        numeric("FlaskRecoveryRate", "INC", 12.5),
        numeric("FlaskAdditionalLifeRecovery", "BASE", 9.5),
        numeric("FlaskCharges", "BASE", 2.25),
        numeric("FlaskCharges", "INC", 17.5),
        numeric("FlaskChargesUsed", "INC", 13.75),
        numeric("FlaskChargesGenerated", "BASE", 2.5),
        numeric("FlaskChargesGained", "INC", 3.25),
        numeric("FlaskChargeRecovery", "INC", 4.5),
        numeric("FlaskEffect", "INC", 6.25),
        numeric("CharmEffect", "INC", 8.25),
        numeric("LocalEffect", "INC", 7.5),
    ];
    recovery.push(modifier(
        "LifeFlaskEffectNotRemoved",
        "FLAG",
        Field::Boolean(true),
    ));
    recovery.push(modifier(
        "ManaFlaskEffectNotRemoved",
        "FLAG",
        Field::Boolean(true),
    ));
    for base in ["Ultimate Life Flask", "Ultimate Mana Flask", "Ruby Charm"] {
        derived.push(cx.derived(
            &format!("{base} recovery duration charges effects"),
            base,
            recovery.clone(),
            Expected::Complete,
        ));
    }
    derived.push(cx.derived(
        "armour late missing override key Source error",
        "Painted Tower Shield",
        vec![
            numeric("Armour", "BASE", 12.5),
            override_value("ArmourData", None, Some(Field::Number(1.0))),
        ],
        Expected::SourceError,
    ));
    derived.push(cx.derived(
        "flask late missing override key Source error",
        "Ultimate Life Flask",
        vec![
            numeric("FlaskCharges", "BASE", 2.5),
            override_value("FlaskData", None, Some(Field::Number(1.0))),
        ],
        Expected::SourceError,
    ));
    derived.push(cx.derived(
        "charm late missing override key Source error",
        "Ruby Charm",
        vec![
            numeric("Duration", "INC", 12.5),
            override_value("CharmData", None, Some(Field::Number(1.0))),
        ],
        Expected::SourceError,
    ));
    derived.push(cx.derived(
        "flask zero recovery rate nonfinite frontier",
        "Ultimate Life Flask",
        vec![numeric("FlaskRecoveryRate", "INC", -100.0)],
        Expected::NonFiniteFrontier,
    ));
    json!({"parser_driven_fresh":fresh,"parser_driven_cross_family_reparse":reparse,"parser_driven_no_base_reparse":no_base_reparse,"derived_modifier_inputs":derived,
        "scope":{"whole_local_data_graphs":true,"nested_override_aliases_compared":true,
            "actual_dependency_arity_observed":false,"dependency_order_parity":false,
            "source_outputs_injected_as_native_results":false,"original_saved_builds_mutated":false}})
}
