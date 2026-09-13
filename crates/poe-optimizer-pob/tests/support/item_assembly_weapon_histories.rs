//! Weapon cases invoke full original methods. Derived modifiers are explicit
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
    QualitySourceError,
    TotalDpsSourceError,
    NonFiniteFrontier(&'static str),
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
        if matches!(expected, Expected::QualitySourceError) {
            before.raw_set("quality", false).unwrap();
            request.state.retained_fields.insert(
                "quality".into(),
                poe_optimizer_import::item_loading::ItemScalar::Boolean(false),
            );
        }
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
                assert_eq!(
                    before.raw_get::<Table>("weaponData").unwrap().raw_len(),
                    2,
                    "complete original weapon assembly must visit both slots"
                );
                json!({"status":"complete", "graph":compare_graph(&result, &event, label)})
            }
            Expected::SourceError
            | Expected::QualitySourceError
            | Expected::TotalDpsSourceError => {
                let source_error = source_result.unwrap_err().to_string();
                assert!(!event.raw_get::<bool>("completed").unwrap());
                if matches!(expected, Expected::TotalDpsSourceError) {
                    let first: Table = before
                        .raw_get::<Table>("weaponData")
                        .unwrap()
                        .raw_get(1)
                        .unwrap();
                    assert_eq!(
                        first.raw_get::<f64>("TotalDPS").unwrap(),
                        0.0,
                        "TotalDPS failure occurs after resetting the prior override"
                    );
                }
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
            Expected::NonFiniteFrontier(field) => {
                source_result.unwrap();
                assert!(event.raw_get::<bool>("completed").unwrap());
                let original: f64 = before
                    .raw_get::<Table>("weaponData")
                    .unwrap()
                    .raw_get::<Table>(1)
                    .unwrap()
                    .raw_get(field)
                    .unwrap();
                assert!(
                    !original.is_finite(),
                    "derived input must reach a nonfinite original field"
                );
                let native_error = attempt.result.unwrap_err();
                assert_eq!(native_error.kind, assembly::AssemblyErrorKind::Unsupported);
                assert!(attempt.partial.as_ref().is_some_and(|p| !p.is_complete()));
                json!({"status":"explicit_nonfinite_frontier","source_completed":true,
                    "source_field":field,"source_field_bits":format!("{:016x}",original.to_bits()),
                    "native_error":native_error.message,"stage":attempt.stage,
                    "final_graph_parity_claimed":false})
            }
        };
        json!({"label":label,"base":base,"scope":"derived finite pre-call modifier input; not a parser-return claim",
            "input_modifiers":modifiers,"ordered_input_graph_sha256":input_sha,
            "same_ordered_native_source_inputs_verified":true,"source_poststate_used_as_dependency":false,
            "quality_input":if matches!(expected,Expected::QualitySourceError){json!(false)}else{json!(17)},
            "unreachable_slot_local_graph_observed":false,
            "registration_authorized":false,"outcome":outcome})
    }
}

fn flags(mut row: Metadata, flags: Field, keywords: Field) -> Metadata {
    row.fields.insert("flags".into(), flags);
    row.fields.insert("keywordFlags".into(), keywords);
    row
}
fn condition(name: &str) -> Field {
    Field::Table(record([
        ("type", Field::Text("Condition".into())),
        ("var", Field::Text(name.into())),
    ]))
}
fn tagged(mut row: Metadata, tags: Vec<Field>) -> Metadata {
    row.indexed
        .extend(tags.into_iter().enumerate().map(|(i, v)| (i as i64 + 1, v)));
    row
}
fn original_damage_order(lua: &Lua, class: &Table) -> (Function, Table, Vec<String>) {
    let method: Function = class.raw_get("BuildModListForSlotNum").unwrap();
    let get: Function = lua
        .globals()
        .raw_get::<Table>("debug")
        .unwrap()
        .raw_get("getupvalue")
        .unwrap();
    let mut result = None;
    let mut ended = false;
    for i in 1..=256 {
        let (name, value): (Option<String>, Value) = get.call((method.clone(), i)).unwrap();
        let Some(name) = name else {
            ended = true;
            break;
        };
        if name == "dmgTypeList" {
            assert!(result.is_none(), "ambiguous original damage order capture");
            let Value::Table(value) = value else {
                panic!("damage order capture type")
            };
            result = Some(value);
        }
    }
    assert!(ended, "bounded original upvalue inventory");
    let table = result.expect("original damage order capture");
    assert_eq!(table.raw_len(), 5);
    let order = (1..=5)
        .map(|i| table.raw_get::<String>(i).unwrap())
        .collect();
    (method, table, order)
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
    let mod_flags: Table = lua.globals().raw_get("ModFlag").unwrap();
    let keyword_flags: Table = lua.globals().raw_get("KeywordFlag").unwrap();
    let attack: f64 = mod_flags.raw_get("Attack").unwrap();
    let spell: f64 = mod_flags.raw_get("Spell").unwrap();
    let keyword_attack: f64 = keyword_flags.raw_get("Attack").unwrap();
    assert!(attack > 0.0 && spell > 0.0 && keyword_attack > 0.0);
    let (slot_method, damage_table, damage_order) = original_damage_order(lua, class);
    let mut fresh = Vec::new();
    for (base, quality) in [
        ("Wooden Club", 0),
        ("Crude Bow", 17),
        ("Makeshift Crossbow", 21),
        ("Wrapped Quarterstaff", 19),
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
        assert_eq!(source.raw_get::<Table>("weaponData").unwrap().raw_len(), 2);
        fresh.push(
            json!({"base":base,"quality":quality,"parse":parsed,"final":finished,"weapon_slots":2}),
        );
    }
    let source = item(lua);
    let mut native = machine(snapshot);
    let mut p = provider(snapshot, parser);
    let mut prior_weapon: Option<Table> = None;
    let mut cross = Vec::new();
    for (base, is_weapon) in [
        ("Makeshift Crossbow", true),
        ("Painted Tower Shield", false),
        ("Gold Ring", false),
        ("Wooden Club", true),
        ("Ultimate Life Flask", false),
        ("Crude Bow", true),
    ] {
        let (parsed, _) = parse_step(
            lua,
            module,
            &source,
            parse,
            &mut native,
            &mut p,
            &raw(base, 13, ""),
            base,
        );
        let finished = finish_step(lua, module, &source, build, &mut native, &mut p, base);
        let current: Table = source.raw_get("weaponData").unwrap();
        if let Some(prior) = &prior_weapon {
            if is_weapon {
                assert_ne!(
                    prior, &current,
                    "weapon branch must allocate a fresh weaponData table"
                );
            } else {
                assert_eq!(
                    prior, &current,
                    "other families preserve previous weaponData identity"
                );
            }
        }
        prior_weapon = Some(current);
        cross.push(
            json!({"base":base,"weapon_branch":is_weapon,"parse":parsed,"final":finished,
            "original_weapon_data_lifetime_verified":true}),
        );
    }
    let mut derived = Vec::new();
    let mut formula = vec![
        flags(
            numeric("Speed", "INC", 11.25),
            Field::Number(attack),
            Field::Number(0.0),
        ),
        flags(
            numeric("ReloadSpeed", "INC", 17.5),
            Field::Number(attack),
            Field::Number(0.0),
        ),
        numeric("AlternateQualityLocalAttackSpeedPer8Quality", "INC", 3.25),
        numeric("WeaponRange", "BASE", 1.75),
        numeric("WeaponRangeMetre", "BASE", 0.25),
        numeric("AlternateQualityLocalWeaponRangePer10Quality", "BASE", 2.5),
        numeric("LocalElementalDamage", "INC", 13.75),
        numeric("PhysicalDamage", "INC", 17.5),
        numeric("CritChance", "BASE", 1.25),
        numeric("CritChance", "INC", 19.5),
        numeric("AlternateQualityLocalCritChancePer4Quality", "INC", 2.75),
    ];
    for (index, damage) in damage_order.iter().enumerate() {
        formula.push(numeric(
            &format!("{damage}Min"),
            "BASE",
            2.25 + index as f64,
        ));
        formula.push(numeric(
            &format!("{damage}Max"),
            "BASE",
            5.75 + index as f64,
        ));
        if damage != "Physical" && damage != "Chaos" {
            formula.push(numeric(
                &format!("Local{damage}Damage"),
                "INC",
                7.5 + index as f64,
            ));
        }
    }
    derived.push(cx.derived(
        "all weapon channels reload range and crit",
        "Makeshift Crossbow",
        formula.clone(),
        Expected::Complete,
    ));
    formula.push(numeric("AlternateQualityWeapon", "BASE", 1.0));
    derived.push(cx.derived(
        "alternate weapon quality suppresses physical scalar",
        "Wooden Club",
        formula,
        Expected::Complete,
    ));
    derived.push(cx.derived(
        "nonpositive endpoints omit channel outputs",
        "Wooden Club",
        vec![
            numeric("PhysicalMin", "BASE", -1000.0),
            numeric("FireMin", "BASE", 0.0),
            numeric("FireMax", "BASE", 9.0),
            numeric("ChaosMin", "BASE", 2.25),
            numeric("ChaosMax", "BASE", 3.75),
        ],
        Expected::Complete,
    ));
    derived.push(cx.derived(
        "zero attack rate remains finite",
        "Wooden Club",
        vec![flags(
            numeric("Speed", "INC", -100.0),
            Field::Number(attack),
            Field::Number(0.0),
        )],
        Expected::Complete,
    ));
    let nested = Field::Table(record([
        ("label", Field::Text("weapon override alias".into())),
        (
            "values",
            Field::Array(vec![Field::Boolean(false), Field::Number(2.5)]),
        ),
    ]));
    derived.push(cx.derived(
        "ordered weapon overrides nil deletion nested alias and final total",
        "Wooden Club",
        vec![
            override_value("WeaponData", Some("PhysicalDPS"), Some(Field::Number(12.5))),
            override_value(
                "WeaponData",
                Some("PhysicalDPS"),
                Some(Field::Text("7.25".into())),
            ),
            override_value("WeaponData", Some("AttackRate"), None),
            override_value(
                "WeaponData",
                Some("TotalDPS"),
                Some(Field::Text("overwritten before sum".into())),
            ),
            override_value("WeaponData", Some("custom"), Some(nested)),
            override_value(
                "WeaponData",
                Some("FalseField"),
                Some(Field::Boolean(false)),
            ),
            numeric("Accuracy", "BASE", 1.0),
        ],
        Expected::Complete,
    ));
    derived.push(cx.derived(
        "nil DPS override contributes zero to final total",
        "Wooden Club",
        vec![override_value("WeaponData", Some("PhysicalDPS"), None)],
        Expected::Complete,
    ));
    // 108 exact flag/keyword combinations plus bounded tag and primitive-type
    // edge cases. All remain whole original modifier records in both slots.
    let mut residual = Vec::new();
    for name in [
        "Accuracy",
        "CritMultiplier",
        "ImpaleChance",
        "LifeOnHit",
        "ManaOnHit",
        "PhysicalDamageLifeLeech",
        "PhysicalDamageManaLeech",
        "PoisonChance",
        "BleedChance",
    ] {
        for flag in [0.0, attack, spell, attack + spell] {
            for keyword in [0.0, keyword_attack, keyword_attack + 1.0] {
                residual.push(flags(
                    numeric(name, "BASE", 1.0),
                    Field::Number(flag),
                    Field::Number(keyword),
                ));
            }
        }
    }
    for name in ["PoisonChance", "BleedChance"] {
        for tags in [
            vec![condition("CriticalStrike")],
            vec![condition("OtherCondition")],
            vec![condition("CriticalStrike"), condition("AnotherCondition")],
            vec![Field::Table(record([
                ("type", Field::Text("SlotNumber".into())),
                ("num", Field::Number(1.0)),
            ]))],
        ] {
            residual.push(tagged(numeric(name, "BASE", 1.0), tags));
        }
    }
    residual.push(tagged(
        numeric("Accuracy", "BASE", 1.0),
        vec![condition("CriticalStrike")],
    ));
    let mut missing_keyword = numeric("Accuracy", "BASE", 1.0);
    missing_keyword.fields.remove("keywordFlags");
    residual.push(missing_keyword);
    let mut missing_flag = numeric("ImpaleChance", "BASE", 1.0);
    missing_flag.fields.remove("flags");
    residual.push(missing_flag);
    residual.push(flags(
        numeric("PoisonChance", "BASE", 1.0),
        Field::Boolean(false),
        Field::Text("ignored by poison branch".into()),
    ));
    residual.push(flags(
        numeric("PoisonChance", "BASE", 1.0),
        Field::Text("0".into()),
        Field::Boolean(false),
    ));
    residual.push(flags(
        numeric("Accuracy", "BASE", 1.0),
        Field::Text("0".into()),
        Field::Number(0.0),
    ));
    let residual_count = residual.len();
    assert_eq!(residual_count, 122);
    derived.push(cx.derived(
        "residual exact flags keywords tags and both hands",
        "Wooden Club",
        residual,
        Expected::Complete,
    ));
    derived.push(cx.derived(
        "late WeaponData nil key fails before TotalDPS",
        "Wooden Club",
        vec![override_value("WeaponData", None, Some(Field::Number(1.0)))],
        Expected::SourceError,
    ));
    derived.push(cx.derived(
        "late WeaponData scalar payload fails",
        "Wooden Club",
        vec![modifier("WeaponData", "LIST", Field::Number(7.0))],
        Expected::SourceError,
    ));
    derived.push(cx.derived(
        "TotalDPS fails after override and zero initialization",
        "Wooden Club",
        vec![
            override_value(
                "WeaponData",
                Some("PhysicalDPS"),
                Some(Field::Text("not a number".into())),
            ),
            override_value("WeaponData", Some("TotalDPS"), Some(Field::Number(999.0))),
            numeric("Accuracy", "BASE", 1.0),
        ],
        Expected::TotalDpsSourceError,
    ));
    derived.push(cx.derived(
        "quality failure consumes Speed before alternate-quality query",
        "Wooden Club",
        vec![
            flags(
                numeric("Speed", "INC", 11.25),
                Field::Number(attack),
                Field::Number(0.0),
            ),
            numeric("AlternateQualityLocalAttackSpeedPer8Quality", "INC", 3.25),
        ],
        Expected::QualitySourceError,
    ));
    derived.push(cx.derived(
        "zero reload divisor explicit nonfinite frontier",
        "Makeshift Crossbow",
        vec![flags(
            numeric("ReloadSpeed", "INC", -100.0),
            Field::Number(attack),
            Field::Number(0.0),
        )],
        Expected::NonFiniteFrontier("ReloadTime"),
    ));
    let (after_method, after_table, after_order) = original_damage_order(lua, class);
    assert_eq!(slot_method, after_method);
    assert_eq!(damage_table, after_table);
    assert_eq!(damage_order, after_order);
    assert_eq!(
        lua.globals().raw_get::<Table>("ModFlag").unwrap(),
        mod_flags
    );
    assert_eq!(
        lua.globals().raw_get::<Table>("KeywordFlag").unwrap(),
        keyword_flags
    );
    assert_eq!(mod_flags.raw_get::<f64>("Attack").unwrap(), attack);
    assert_eq!(mod_flags.raw_get::<f64>("Spell").unwrap(), spell);
    assert_eq!(
        keyword_flags.raw_get::<f64>("Attack").unwrap(),
        keyword_attack
    );
    json!({"parser_driven_fresh":fresh,"parser_driven_cross_family_reparse":cross,"derived_modifier_inputs":derived,
        "original_constants":{"attack_flag":attack,"spell_flag":spell,"attack_keyword":keyword_attack,"damage_order":damage_order},
        "residual_input_records":residual_count,"scope":{"whole_weapon_data_graph":true,"complete_cases_require_both_slots":true,
            "actual_dependency_arity_observed":false,"dependency_order_parity":false,"unreachable_failure_frame_locals_observed":false,
            "source_outputs_used_as_dependencies":false,"original_saved_builds_mutated":false}})
}
