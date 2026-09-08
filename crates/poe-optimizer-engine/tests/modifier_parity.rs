//! Oracle executes pinned ModStore/ModDB methods and their actual upstream helpers.
#![cfg(not(target_arch = "wasm32"))]

use std::collections::BTreeMap;

use mlua::{Function, Lua, Table};
use poe_optimizer_engine::conditions::{
    ConditionActor, ConditionEnvironment, ConditionEnvironmentInput, ConditionVariables,
    Conditions, ModifierTag, WeaponConditions,
};
use poe_optimizer_engine::modifiers::{
    KEYWORD_MATCH_ALL, ModifierDatabase, ModifierError, ModifierInput, ModifierKind, ModifierValue,
    MorePrecision, NumericKind, QueryContext, SUPPORTED_MOD_FLAG_BITS, SumKind,
    TaggedModifierInput,
};
use sha2::{Digest, Sha256};

const DB: &str = include_str!("../../../vendor/path-of-building-poe2/src/Classes/ModDB.lua");
const STORE: &str = include_str!("../../../vendor/path-of-building-poe2/src/Classes/ModStore.lua");
const GLOBAL: &str = include_str!("../../../vendor/path-of-building-poe2/src/Data/Global.lua");
const COMMON: &str = include_str!("../../../vendor/path-of-building-poe2/src/Modules/Common.lua");
const DATA: &str = include_str!("../../../vendor/path-of-building-poe2/src/Modules/Data.lua");
const MODTOOLS: &str =
    include_str!("../../../vendor/path-of-building-poe2/src/Modules/ModTools.lua");

struct Oracle {
    lua: Lua,
    make_db: Function,
    query: Function,
    warm: bool,
}

fn section<'a>(source: &'a str, begin: &str, end: &str) -> &'a str {
    let start = source.find(begin).unwrap();
    &source[start..start + source[start..].find(end).unwrap()]
}

impl Oracle {
    fn new(warm: bool) -> Self {
        for (name, source, expected) in [
            (
                "ModDB",
                DB,
                "1417e208c10466395d67a52ec9f3f52719ec16e50760ef06760fb22207a1eab6",
            ),
            (
                "ModStore",
                STORE,
                "432bcffa24f1f2a232499d0a01b5ba01fe4adc259318f19f16cdddd99afe8c62",
            ),
            (
                "Global",
                GLOBAL,
                "1482a574c9b8a06a4734577e549bd87917e5cd631523708d6f2c2fa62d62db2c",
            ),
            (
                "Common",
                COMMON,
                "bae6d0704a92fb04ed56a6033f9229eb2683c53785c14b9c5571b4c0591b0fe8",
            ),
            (
                "Data",
                DATA,
                "2c7d37cfdeda234a8741847e6dd5019dcf8ca9752aaed596f333f80489b430a4",
            ),
        ] {
            assert_eq!(
                format!("{:x}", Sha256::digest(source.replace("\r\n", "\n"))),
                expected,
                "Review native modifier translation after changing {name}"
            );
        }
        assert_eq!(
            format!("{:x}", Sha256::digest(MODTOOLS.replace("\r\n", "\n"))),
            "1ebd614ca55c052cb0be6dd0d16c8b9a944540a47ad50eca607c21d8913e1246"
        );
        let lua = Lua::new();
        let common = COMMON.replace("\r\n", "\n");
        // The actual class library and round definition are extracted, with only
        // captured standard-library locals supplied. No calculation is rewritten.
        lua.load(format!(
            "local s_format = string.format; local m_floor = math.floor; common = {{}}\n{}\n{}",
            section(&common, "-- Class library\n", "function codePointToUTF8"),
            section(
                &common,
                "function round(val, dec)\n",
                "\n--- Rounds down a number"
            ),
        ))
        .set_name("pinned-Common-class-and-round")
        .exec()
        .unwrap();
        lua.load(section(
            &common,
            "function copyTable(tbl, noRecurse)\n",
            "\ndo\n",
        ))
        .set_name("pinned-Common-copyTable")
        .exec()
        .unwrap();
        lua.load(GLOBAL).set_name("pinned-Global").exec().unwrap();
        lua.load("modLib = {}; data = {};").exec().unwrap();
        let modtools = MODTOOLS.replace("\r\n", "\n");
        lua.load(section(
            &modtools,
            "function modLib.createMod(",
            "\nmodLib.parseMod,",
        ))
        .set_name("pinned-ModTools-createMod")
        .exec()
        .unwrap();
        let data = DATA.replace("\r\n", "\n");
        lua.load(section(
            &data,
            "data.highPrecisionMods = {",
            "data.weaponTypeInfo = {",
        ))
        .set_name("pinned-Data-highPrecisionMods")
        .exec()
        .unwrap();
        lua.load(STORE).set_name("pinned-ModStore").exec().unwrap();
        lua.load(DB).set_name("pinned-ModDB").exec().unwrap();
        lua.load(if warm { "jit.on()" } else { "jit.off()" })
            .exec()
            .unwrap();
        let make_db = lua
            .load(
                "return function(layers) local parent; for i = #layers, 1, -1 do \
             local db = new('ModDB'):ModDB(parent); \
             for _, mod in ipairs(layers[i]) do db:AddMod(mod) end; parent = db end; \
             return parent or new('ModDB'):ModDB() end",
            )
            .eval()
            .unwrap();
        let query = lua
            .load(
                "return function(db, cfg, names, kind, warm) local value; \
             for i = 1, (warm and 200 or 1) do \
               if kind == 'MORE' then value = db:More(cfg, unpack(names)) \
               elseif kind == 'OVERRIDE' then value = db:Override(cfg, unpack(names)) \
               else value = db:Sum(kind, cfg, unpack(names)) end \
             end; return value end",
            )
            .eval()
            .unwrap();
        Self {
            lua,
            make_db,
            query,
            warm,
        }
    }

    fn database(&self, layers: &[Vec<ModifierInput>]) -> Table {
        let tables = self.lua.create_table().unwrap();
        for (i, inputs) in layers.iter().enumerate() {
            let layer = self.lua.create_table().unwrap();
            for (j, input) in inputs.iter().enumerate() {
                let ModifierKind::Numeric(kind) = input.kind else {
                    panic!("oracle numeric only")
                };
                let ModifierValue::Number(value) = input.value else {
                    panic!("oracle number only")
                };
                assert!(input.tag_kinds.is_empty());
                let modifier = self.lua.create_table().unwrap();
                modifier.set("name", input.name.as_str()).unwrap();
                modifier.set("type", kind.upstream_name()).unwrap();
                modifier.set("value", value).unwrap();
                modifier.set("flags", input.flags as f64).unwrap();
                modifier
                    .set("keywordFlags", input.keyword_flags as f64)
                    .unwrap();
                modifier.set("source", input.source.as_deref()).unwrap();
                layer.set(j + 1, modifier).unwrap();
            }
            tables.set(i + 1, layer).unwrap();
        }
        self.make_db.call(tables).unwrap()
    }

    fn calculate(
        &self,
        db: &Table,
        context: &QueryContext,
        names: &[&str],
        kind: NumericKind,
    ) -> mlua::Result<Option<f64>> {
        let config = self.lua.create_table()?;
        config.set("flags", context.flags as f64)?;
        config.set("keywordFlags", context.keyword_flags as f64)?;
        config.set("source", context.source.as_deref())?;
        let names = self.lua.create_sequence_from(names.iter().copied())?;
        self.query
            .call((db.clone(), config, names, kind.upstream_name(), self.warm))
    }
}

fn modifier(name: &str, kind: NumericKind, value: f64) -> ModifierInput {
    ModifierInput {
        name: name.to_owned(),
        kind: ModifierKind::Numeric(kind),
        value: ModifierValue::Number(value),
        flags: 0,
        keyword_flags: 0,
        source: Some("Item:42".to_owned()),
        tag_kinds: vec![],
    }
}

fn assert_number(actual: Option<f64>, expected: Option<f64>) {
    match (actual, expected) {
        (None, None) => {}
        (Some(actual), Some(expected)) if expected.is_nan() => assert!(actual.is_nan()),
        (Some(actual), Some(expected)) if expected == 0.0 || expected.is_infinite() => {
            assert_eq!(
                actual.to_bits(),
                expected.to_bits(),
                "{actual:?} != {expected:?}"
            );
        }
        (Some(actual), Some(expected)) => {
            let tolerance = 1e-12 * expected.abs().max(1.0);
            assert!(
                actual.is_finite() && (actual - expected).abs() <= tolerance,
                "{actual:?} != {expected:?} (tolerance {tolerance})"
            );
        }
        pair => panic!("Different availability: {pair:?}"),
    }
}

fn compare_all(
    oracle: &Oracle,
    layers: &[Vec<ModifierInput>],
    contexts: &[QueryContext],
    names: &[&str],
    precision: &MorePrecision,
) {
    let native = ModifierDatabase::try_new(layers.to_vec()).unwrap();
    let db = oracle.database(layers);
    for context in contexts {
        for kind in [
            NumericKind::Base,
            NumericKind::Increased,
            NumericKind::More,
            NumericKind::Override,
        ] {
            let result = match kind {
                NumericKind::Base => Some(native.sum(SumKind::Base, context, names).unwrap()),
                NumericKind::Increased => {
                    Some(native.sum(SumKind::Increased, context, names).unwrap())
                }
                NumericKind::More => Some(native.more(context, names, precision).unwrap()),
                NumericKind::Override => native.override_value(context, names).unwrap(),
            };
            assert_number(result, oracle.calculate(&db, context, names, kind).unwrap());
        }
    }
}

#[test]
fn flags_keyword_any_all_and_parent_layers_match_actual_queries() {
    let flags = [0, 1, 2, 3, 1_u64 << 32, 1_u64 << 52, (1_u64 << 52) | 3];
    let keywords = [
        0,
        1,
        2,
        3,
        KEYWORD_MATCH_ALL,
        KEYWORD_MATCH_ALL | 1,
        KEYWORD_MATCH_ALL | 3,
    ];
    let mut layers = vec![vec![], vec![], vec![]];
    for (i, flag) in flags.iter().enumerate() {
        for (j, keyword) in keywords.iter().enumerate() {
            for kind in [
                NumericKind::Base,
                NumericKind::Increased,
                NumericKind::More,
                NumericKind::Override,
            ] {
                let mut entry = modifier("Damage", kind, ((i * 7 + j) as f64 - 20.0) / 10.0);
                entry.flags = *flag;
                entry.keyword_flags = *keyword;
                layers[(i + j) % 3].push(entry);
            }
        }
    }
    let contexts: Vec<_> = flags
        .into_iter()
        .flat_map(|flags| {
            keywords.into_iter().map(move |keyword_flags| QueryContext {
                flags,
                keyword_flags,
                source: None,
            })
        })
        .collect();
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        let upstream_flags: Table = oracle.lua.globals().get("ModFlag").unwrap();
        for pair in upstream_flags.pairs::<String, f64>() {
            let (name, bits) = pair.unwrap();
            assert_eq!(
                (bits as u64) & !SUPPORTED_MOD_FLAG_BITS,
                0,
                "{name} unsupported"
            );
        }
        compare_all(
            &oracle,
            &layers,
            &contexts,
            &["Damage", "Missing", "Damage"],
            &MorePrecision::pinned(),
        );
    }
}

#[test]
fn source_filters_and_override_precedence_match_actual_queries() {
    let mut layers = vec![vec![], vec![], vec![]];
    for (i, source) in ["Item:42", "Tree:17", ":Item:42", "", "::", "λ:42"]
        .into_iter()
        .enumerate()
    {
        for kind in [
            NumericKind::Base,
            NumericKind::Increased,
            NumericKind::More,
            NumericKind::Override,
        ] {
            let mut entry = modifier(if i % 2 == 0 { "A" } else { "B" }, kind, i as f64);
            entry.source = Some(source.to_owned());
            layers[i % 3].push(entry);
        }
    }
    let contexts: Vec<_> = [
        None,
        Some("Item"),
        Some("Item:42"),
        Some("Tree"),
        Some(""),
        Some(":"),
        Some("λ"),
        Some("Missing"),
    ]
    .into_iter()
    .map(|source| QueryContext {
        source: source.map(str::to_owned),
        ..Default::default()
    })
    .collect();
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for names in [
            vec![],
            vec!["A"],
            vec!["B", "A"],
            vec!["A", "B"],
            vec!["A"; 8],
        ] {
            compare_all(
                &oracle,
                &layers,
                &contexts,
                &names,
                &MorePrecision::pinned(),
            );
        }
    }
    let native = ModifierDatabase::try_new(layers).unwrap();
    assert_eq!(
        native
            .override_value(&QueryContext::default(), &["A"])
            .unwrap(),
        Some(0.0)
    );
    assert_eq!(native.layer(0).unwrap()[0].source(), Some("Item:42"));
}

#[test]
fn more_rounding_precision_carry_and_parent_grouping_match_actual_queries() {
    let precision = MorePrecision::pinned();
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        let data: Table = oracle.lua.globals().get("data").unwrap();
        let high_precision: Table = data.get("highPrecisionMods").unwrap();
        let mut more_settings = BTreeMap::new();
        for pair in high_precision.pairs::<String, Table>() {
            let (name, settings) = pair.unwrap();
            if let Some(places) = settings.get::<Option<u8>>("MORE").unwrap() {
                more_settings.insert(name, places);
            }
        }
        assert_eq!(MorePrecision::try_new(more_settings).unwrap(), precision);
        for value in [
            -200.5, -100.5, -100.0, -99.5, -0.5, 0.0, 0.5, 0.51, 1.1, 3.37, 12.5, 35.0, 100.0,
        ] {
            let layers = vec![
                vec![
                    modifier("Damage", NumericKind::More, value),
                    modifier("Damage", NumericKind::More, 3.37),
                    modifier("SupportManaMultiplier", NumericKind::More, 3.37),
                ],
                vec![
                    modifier("Damage", NumericKind::More, value.next_up()),
                    modifier("ReservationMultiplier", NumericKind::More, -12.5),
                ],
                vec![modifier("Damage", NumericKind::More, value.next_down())],
            ];
            for names in [
                vec!["Damage"],
                vec!["SupportManaMultiplier", "Damage"],
                vec!["Damage", "SupportManaMultiplier"],
                vec!["ReservationMultiplier", "Missing", "Damage", "Damage"],
            ] {
                compare_all(
                    &oracle,
                    &layers,
                    &[QueryContext::default()],
                    &names,
                    &precision,
                );
            }
        }
        // An alternate explicit data snapshot exercises decimal zero and carried
        // higher precision, without replacing any upstream aggregation function.
        let settings = BTreeMap::from([
            ("A".to_owned(), 0),
            ("B".to_owned(), 2),
            ("C".to_owned(), 15),
        ]);
        let custom = oracle.lua.create_table().unwrap();
        for (name, places) in &settings {
            let entry = oracle.lua.create_table().unwrap();
            entry.set("MORE", *places).unwrap();
            custom.set(name.as_str(), entry).unwrap();
        }
        data.set("highPrecisionMods", custom).unwrap();
        let custom_precision = MorePrecision::try_new(settings).unwrap();
        let layers = vec![vec![
            modifier("A", NumericKind::More, 43.5),
            modifier("B", NumericKind::More, -150.1),
            modifier("C", NumericKind::More, 3.37),
        ]];
        compare_all(
            &oracle,
            &layers,
            &[QueryContext::default()],
            &["B", "A", "C", "Missing"],
            &custom_precision,
        );
    }
}

#[test]
fn exceptional_arithmetic_and_grouped_cancellation_match_actual_queries() {
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for value in [
            -0.0,
            0.0,
            f64::MIN_POSITIVE,
            f64::MAX,
            f64::MIN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ] {
            let mut layers = vec![vec![], vec![], vec![]];
            for kind in [
                NumericKind::Base,
                NumericKind::Increased,
                NumericKind::More,
                NumericKind::Override,
            ] {
                layers[0].push(modifier("A", kind, value));
                layers[1].push(modifier("A", kind, -1e16));
                layers[2].push(modifier("A", kind, 1.0));
            }
            compare_all(
                &oracle,
                &layers,
                &[QueryContext::default()],
                &["A"],
                &MorePrecision::pinned(),
            );
        }
        let layers = vec![
            vec![modifier("A", NumericKind::Base, 1e16)],
            vec![modifier("A", NumericKind::Base, -1e16)],
            vec![modifier("A", NumericKind::Base, 1.0)],
        ];
        compare_all(
            &oracle,
            &layers,
            &[QueryContext::default()],
            &["A"],
            &MorePrecision::pinned(),
        );
        let native = ModifierDatabase::try_new(layers).unwrap();
        assert_eq!(
            native
                .sum(SumKind::Base, &QueryContext::default(), &["A"])
                .unwrap(),
            0.0
        );
    }
}

#[test]
fn unsupported_tags_values_masks_and_query_limits_fail_explicitly() {
    for tag in [
        "Condition",
        "ActorCondition",
        "Multiplier",
        "PerStat",
        "SkillName",
        "SocketedIn",
        "FutureUnknownTag",
    ] {
        let mut entry = modifier("UnqueriedStat", NumericKind::Base, 1.0);
        entry.tag_kinds.push(tag.to_owned());
        assert!(matches!(
            ModifierDatabase::try_new(vec![vec![], vec![entry]]),
            Err(ModifierError::UnsupportedTag {
                layer: 1,
                modifier: 0,
                ..
            })
        ));
    }
    for kind in ["FLAG", "LIST", "MAX", "UNKNOWN"] {
        let mut entry = modifier("A", NumericKind::Base, 1.0);
        entry.kind = ModifierKind::Unsupported(kind.to_owned());
        assert!(matches!(
            ModifierDatabase::try_new(vec![vec![entry]]),
            Err(ModifierError::UnsupportedKind { .. })
        ));
    }
    for kind in ["nil", "boolean", "string", "table", "function"] {
        let mut entry = modifier("A", NumericKind::Base, 1.0);
        entry.value = ModifierValue::Unsupported {
            kind: kind.to_owned(),
        };
        assert!(matches!(
            ModifierDatabase::try_new(vec![vec![entry]]),
            Err(ModifierError::UnsupportedValue { .. })
        ));
    }
    let native = ModifierDatabase::default();
    for (flags, keyword_flags) in [(1 << 31, 0), (1 << 53, 0), (0, 1 << 31), (u64::MAX, 0)] {
        let context = QueryContext {
            flags,
            keyword_flags,
            source: None,
        };
        assert!(matches!(
            native.sum(SumKind::Base, &context, &["A"]),
            Err(ModifierError::UnsupportedFlags { .. })
        ));
        let mut entry = modifier("A", NumericKind::Base, 1.0);
        entry.flags = flags;
        entry.keyword_flags = keyword_flags;
        assert!(matches!(
            ModifierDatabase::try_new(vec![vec![entry]]),
            Err(ModifierError::UnsupportedFlags { .. })
        ));
    }
    assert!(matches!(
        native.more(
            &QueryContext::default(),
            &["A"; 9],
            &MorePrecision::pinned()
        ),
        Err(ModifierError::TooManyNames { count: 9 })
    ));
    assert!(matches!(
        MorePrecision::try_new(BTreeMap::from([("A".to_owned(), 16)])),
        Err(ModifierError::UnsupportedPrecision { .. })
    ));
}

#[test]
fn absent_sources_preserve_sum_behavior_and_report_upstream_errors() {
    let oracle = Oracle::new(false);
    let mut layer = vec![];
    for kind in [
        NumericKind::Base,
        NumericKind::Increased,
        NumericKind::More,
        NumericKind::Override,
    ] {
        let mut entry = modifier("A", kind, 12.0);
        entry.source = None;
        layer.push(entry);
    }
    let layers = vec![layer];
    compare_all(
        &oracle,
        &layers,
        &[QueryContext::default()],
        &["A"],
        &MorePrecision::pinned(),
    );
    let native = ModifierDatabase::try_new(layers.clone()).unwrap();
    let db = oracle.database(&layers);
    let context = QueryContext {
        source: Some("Item".to_owned()),
        ..Default::default()
    };
    for (kind, sum_kind) in [
        (NumericKind::Base, SumKind::Base),
        (NumericKind::Increased, SumKind::Increased),
    ] {
        assert_number(
            Some(native.sum(sum_kind, &context, &["A"]).unwrap()),
            oracle.calculate(&db, &context, &["A"], kind).unwrap(),
        );
    }
    assert!(matches!(
        native.more(&context, &["A"], &MorePrecision::pinned()),
        Err(ModifierError::MissingSource { .. })
    ));
    assert!(matches!(
        native.override_value(&context, &["A"]),
        Err(ModifierError::MissingSource { .. })
    ));
    for kind in [NumericKind::More, NumericKind::Override] {
        assert!(oracle.calculate(&db, &context, &["A"], kind).is_err());
    }
    // No matching candidate reaches the source dereference, so no error occurs.
    assert_eq!(
        native
            .more(&context, &["Missing"], &MorePrecision::pinned())
            .unwrap(),
        1.0
    );
}

impl Oracle {
    fn tagged_database(&self, layers: &[Vec<TaggedModifierInput>]) -> Table {
        let plain: Vec<Vec<_>> = layers
            .iter()
            .map(|layer| layer.iter().map(|entry| entry.modifier.clone()).collect())
            .collect();
        let root = self.database(&plain);
        let mut db = root.clone();
        for (layer_index, layer) in layers.iter().enumerate() {
            let mods: Table = db.get("mods").unwrap();
            let mut indexes = BTreeMap::new();
            for input in layer {
                let index = indexes.entry(input.modifier.name.as_str()).or_insert(0);
                *index += 1;
                let bucket: Table = mods.get(input.modifier.name.as_str()).unwrap();
                let entry: Table = bucket.get(*index).unwrap();
                for (i, tag) in input.tags.iter().enumerate() {
                    let table = self.lua.create_table().unwrap();
                    match tag {
                        ModifierTag::Condition { variables, negated } => {
                            table.set("type", "Condition").unwrap();
                            self.variables(&table, variables);
                            table.set("neg", *negated).unwrap();
                        }
                        ModifierTag::ActorCondition {
                            actor,
                            variables,
                            negated,
                        } => {
                            table.set("type", "ActorCondition").unwrap();
                            table.set("actor", actor.as_deref()).unwrap();
                            if let Some(variables) = variables {
                                self.variables(&table, variables);
                            }
                            table.set("neg", *negated).unwrap();
                        }
                        ModifierTag::Unsupported(_) => {
                            panic!("Cannot construct supported oracle input from unknown tag")
                        }
                    }
                    entry.set(i + 1, table).unwrap();
                }
            }
            if layer_index + 1 < layers.len() {
                db = db.get("parent").unwrap();
            }
        }
        root
    }

    fn variables(&self, table: &Table, variables: &ConditionVariables) {
        match variables {
            ConditionVariables::One(name) => table.set("var", name.as_str()).unwrap(),
            ConditionVariables::Any(names) => table
                .set(
                    "varList",
                    self.lua
                        .create_sequence_from(names.iter().map(String::as_str))
                        .unwrap(),
                )
                .unwrap(),
        }
    }

    fn condition_table(&self, values: &Conditions) -> Table {
        let result = self.lua.create_table().unwrap();
        for (name, value) in values {
            result.set(name.as_str(), *value).unwrap();
        }
        result
    }

    fn populate_conditions(&self, root: &Table, layers: &[Conditions]) {
        let mut db = root.clone();
        for (i, layer) in layers.iter().enumerate() {
            db.set("conditions", self.condition_table(layer)).unwrap();
            if i + 1 < layers.len() {
                db = db.get("parent").unwrap();
            }
        }
    }

    fn condition_config(
        &self,
        db: &Table,
        query: &QueryContext,
        input: &ConditionEnvironmentInput,
    ) -> Table {
        self.populate_conditions(db, &input.store_conditions);
        let mut actors = vec![];
        for input in &input.actors {
            let actor = self.lua.create_table().unwrap();
            let layers = vec![vec![]; input.conditions.len()];
            let actor_db = self.database(&layers);
            self.populate_conditions(&actor_db, &input.conditions);
            actor_db.set("actor", actor.clone()).unwrap();
            actor.set("modDB", actor_db).unwrap();
            for (field, weapon) in [
                ("weaponData1", &input.weapon_one),
                ("weaponData2", &input.weapon_two),
            ] {
                let table = self.lua.create_table().unwrap();
                table
                    .set("countsAsAll1H", weapon.counts_as_all_one_handed)
                    .unwrap();
                for (name, value) in &weapon.added {
                    table.set(format!("Added{name}"), *value).unwrap();
                }
                actor.set(field, table).unwrap();
            }
            actors.push(actor);
        }
        for (i, actor) in input.actors.iter().enumerate() {
            for (role, index) in &actor.links {
                actors[i]
                    .set(role.as_str(), actors[*index].clone())
                    .unwrap();
            }
        }
        db.set("actor", actors[input.current_actor].clone())
            .unwrap();
        let config = self.lua.create_table().unwrap();
        config.set("flags", query.flags as f64).unwrap();
        config
            .set("keywordFlags", query.keyword_flags as f64)
            .unwrap();
        config.set("source", query.source.as_deref()).unwrap();
        config
            .set("overrideCond", self.condition_table(&input.overrides))
            .unwrap();
        config
            .set("skillCond", self.condition_table(&input.skill_conditions))
            .unwrap();
        config.set("actor", input.query_actor.as_deref()).unwrap();
        config
    }
}

fn condition(name: &str, negated: bool) -> ModifierTag {
    ModifierTag::Condition {
        variables: ConditionVariables::One(name.to_owned()),
        negated,
    }
}

fn condition_input(layer_count: usize) -> ConditionEnvironmentInput {
    ConditionEnvironmentInput {
        store_conditions: vec![Conditions::new(); layer_count],
        actors: vec![ConditionActor::default()],
        ..Default::default()
    }
}

fn tagged_modifiers(tags: &[Vec<ModifierTag>]) -> Vec<Vec<TaggedModifierInput>> {
    let mut layers = vec![vec![], vec![], vec![]];
    for (i, tags) in tags.iter().enumerate() {
        for kind in [
            NumericKind::Base,
            NumericKind::Increased,
            NumericKind::More,
            NumericKind::Override,
        ] {
            for name in ["A", "SupportManaMultiplier"] {
                let mut modifier = modifier(name, kind, i as f64 + 0.37);
                modifier.flags = (i % 3) as u64;
                modifier.keyword_flags = if i % 2 == 0 { 1 } else { KEYWORD_MATCH_ALL | 3 };
                modifier.source = Some(if i % 2 == 0 { "Item:42" } else { "Tree:17" }.to_owned());
                layers[i % 3].push(TaggedModifierInput {
                    modifier,
                    tags: tags.clone(),
                });
            }
        }
    }
    layers
}

fn compare_conditions(
    oracle: &Oracle,
    layers: &[Vec<TaggedModifierInput>],
    input: &ConditionEnvironmentInput,
) {
    let native = ModifierDatabase::try_new_tagged(layers.to_vec()).unwrap();
    let environment = ConditionEnvironment::try_new(input.clone()).unwrap();
    let db = oracle.tagged_database(layers);
    for query in [
        QueryContext::default(),
        QueryContext {
            flags: 3,
            keyword_flags: 3,
            source: None,
        },
        QueryContext {
            flags: 3,
            keyword_flags: 3,
            source: Some("Item".to_owned()),
        },
        QueryContext {
            flags: 3,
            keyword_flags: 3,
            source: Some("Item:42".to_owned()),
        },
    ] {
        let config = oracle.condition_config(&db, &query, input);
        for names in [
            vec!["A"],
            vec!["SupportManaMultiplier", "A", "A"],
            vec!["A", "Missing", "SupportManaMultiplier"],
        ] {
            for kind in [
                NumericKind::Base,
                NumericKind::Increased,
                NumericKind::More,
                NumericKind::Override,
            ] {
                let expected = oracle
                    .query
                    .call((
                        db.clone(),
                        config.clone(),
                        oracle
                            .lua
                            .create_sequence_from(names.iter().copied())
                            .unwrap(),
                        kind.upstream_name(),
                        oracle.warm,
                    ))
                    .unwrap();
                let actual = match kind {
                    NumericKind::Base => Some(
                        native
                            .sum_with_conditions(SumKind::Base, &query, &names, &environment)
                            .unwrap(),
                    ),
                    NumericKind::Increased => Some(
                        native
                            .sum_with_conditions(SumKind::Increased, &query, &names, &environment)
                            .unwrap(),
                    ),
                    NumericKind::More => Some(
                        native
                            .more_with_conditions(
                                &query,
                                &names,
                                &MorePrecision::pinned(),
                                &environment,
                            )
                            .unwrap(),
                    ),
                    NumericKind::Override => native
                        .override_with_conditions(&query, &names, &environment)
                        .unwrap(),
                };
                assert_number(actual, expected);
            }
        }
    }
}

#[test]
fn conditions_inherit_truthy_parents_and_distinguish_overrides_from_skill_conditions() {
    let tags = vec![
        vec![condition("A", false)],
        vec![condition("A", true)],
        vec![condition("B", false)],
        vec![condition("A", false), condition("B", true)],
        vec![ModifierTag::Condition {
            variables: ConditionVariables::Any(vec!["A".to_owned(), "B".to_owned()]),
            negated: false,
        }],
        vec![ModifierTag::Condition {
            variables: ConditionVariables::Any(vec!["B".to_owned(), "A".to_owned()]),
            negated: true,
        }],
        vec![ModifierTag::Condition {
            variables: ConditionVariables::Any(vec![]),
            negated: true,
        }],
        vec![],
    ];
    let layers = tagged_modifiers(&tags);
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for local in [None, Some(false), Some(true)] {
            for parent in [None, Some(false), Some(true)] {
                for override_value in [None, Some(false), Some(true)] {
                    for skill in [false, true] {
                        let mut input = condition_input(layers.len());
                        if let Some(value) = local {
                            input.store_conditions[0].insert("A".to_owned(), value);
                        }
                        if let Some(value) = parent {
                            input.store_conditions[1].insert("A".to_owned(), value);
                        }
                        input.store_conditions[2].insert("B".to_owned(), true);
                        if let Some(value) = override_value {
                            input.overrides.insert("A".to_owned(), value);
                        }
                        input.skill_conditions.insert("A".to_owned(), skill);
                        compare_conditions(&oracle, &layers, &input);
                    }
                }
            }
        }
    }
}

#[test]
fn actor_conditions_preserve_player_fallback_and_missing_actor_semantics() {
    let mut tags = vec![];
    for actor in [
        None,
        Some("enemy"),
        Some("player"),
        Some("parent"),
        Some("minion"),
        Some("absent"),
    ] {
        for variables in [
            None,
            Some(ConditionVariables::One("A".to_owned())),
            Some(ConditionVariables::Any(vec![
                "B".to_owned(),
                "A".to_owned(),
            ])),
            Some(ConditionVariables::Any(vec![])),
        ] {
            for negated in [false, true] {
                tags.push(vec![ModifierTag::ActorCondition {
                    actor: actor.map(str::to_owned),
                    variables: variables.clone(),
                    negated,
                }]);
            }
        }
    }
    let layers = tagged_modifiers(&tags);
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for direct in [false, true] {
            for parent in [false, true] {
                for enemy in [false, true] {
                    for override_value in [None, Some(false), Some(true)] {
                        let mut input = condition_input(layers.len());
                        input.actors.resize(5, ConditionActor::default());
                        input.store_conditions[0].insert("A".to_owned(), true);
                        input.skill_conditions.insert("B".to_owned(), true); // ActorCondition must ignore this.
                        input.query_actor = Some("absent".to_owned());
                        if let Some(value) = override_value {
                            input.overrides.insert("A".to_owned(), value);
                        }
                        if direct {
                            input.actors[0].links.insert("player".to_owned(), 1);
                        }
                        if parent {
                            input.actors[0].links.insert("parent".to_owned(), 2);
                            input.actors[2].links.insert("player".to_owned(), 3);
                        }
                        if enemy {
                            input.actors[0].links.insert("enemy".to_owned(), 4);
                            input.actors[4].links.insert("player".to_owned(), 4);
                        }
                        input.actors[1].conditions =
                            vec![BTreeMap::from([("A".to_owned(), false)])];
                        input.actors[2].conditions = vec![BTreeMap::from([("B".to_owned(), true)])];
                        input.actors[3].conditions = vec![BTreeMap::from([("A".to_owned(), true)])];
                        input.actors[4].conditions = vec![
                            BTreeMap::from([("A".to_owned(), false)]),
                            BTreeMap::from([("A".to_owned(), true)]),
                        ];
                        compare_conditions(&oracle, &layers, &input);
                    }
                }
            }
        }
    }
}

#[test]
fn all_one_handed_weapon_exceptions_match_condition_tag_ordering() {
    let tags = vec![
        vec![condition("A", true)],
        vec![condition("B", true)],
        vec![condition("A", false)],
        vec![ModifierTag::Condition {
            variables: ConditionVariables::Any(vec!["A".to_owned(), "B".to_owned()]),
            negated: true,
        }],
        vec![ModifierTag::Condition {
            variables: ConditionVariables::Any(vec!["B".to_owned(), "A".to_owned()]),
            negated: true,
        }],
    ];
    let layers = tagged_modifiers(&tags);
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for first in [false, true] {
            for second in [false, true] {
                for added_a in [None, Some(false), Some(true)] {
                    for added_b in [None, Some(false), Some(true)] {
                        let mut input = condition_input(layers.len());
                        input.store_conditions[0] =
                            BTreeMap::from([("A".to_owned(), true), ("B".to_owned(), true)]);
                        let mut added = Conditions::new();
                        if let Some(value) = added_a {
                            added.insert("A".to_owned(), value);
                        }
                        if let Some(value) = added_b {
                            added.insert("B".to_owned(), value);
                        }
                        input.actors[0].weapon_one = WeaponConditions {
                            counts_as_all_one_handed: first,
                            added,
                        };
                        input.actors[0].weapon_two = WeaponConditions {
                            counts_as_all_one_handed: second,
                            added: BTreeMap::from([
                                ("A".to_owned(), false),
                                ("B".to_owned(), true),
                            ]),
                        };
                        compare_conditions(&oracle, &layers, &input);
                    }
                }
            }
        }
    }
}

#[test]
fn conditional_queries_preserve_inactive_more_precision_zero_overrides_and_error_boundaries() {
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for value in [0.0, -0.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1.37] {
            let mut layers = vec![vec![], vec![], vec![]];
            for kind in [
                NumericKind::Base,
                NumericKind::Increased,
                NumericKind::More,
                NumericKind::Override,
            ] {
                layers[0].push(TaggedModifierInput {
                    modifier: modifier("SupportManaMultiplier", kind, value),
                    tags: vec![condition("Active", false)],
                });
                layers[0].push(TaggedModifierInput {
                    modifier: modifier("A", kind, 3.37),
                    tags: vec![],
                });
                layers[1].push(TaggedModifierInput {
                    modifier: modifier("A", kind, -1.63),
                    tags: vec![],
                });
                layers[2].push(TaggedModifierInput {
                    modifier: modifier("SupportManaMultiplier", kind, -0.0),
                    tags: vec![],
                });
            }
            for active in [false, true] {
                let mut input = condition_input(layers.len());
                input.store_conditions[0].insert("Active".to_owned(), active);
                compare_conditions(&oracle, &layers, &input);
            }
        }
        let mut entry = modifier("A", NumericKind::Override, 1.0);
        entry.source = None;
        let layers = vec![vec![TaggedModifierInput {
            modifier: entry,
            tags: vec![condition("Missing", false)],
        }]];
        let native = ModifierDatabase::try_new_tagged(layers.clone()).unwrap();
        let input = condition_input(1);
        let environment = ConditionEnvironment::try_new(input.clone()).unwrap();
        let db = oracle.tagged_database(&layers);
        let query = QueryContext {
            source: Some("Item".to_owned()),
            ..Default::default()
        };
        let config = oracle.condition_config(&db, &query, &input);
        assert!(matches!(
            native.override_with_conditions(&query, &["A"], &environment),
            Err(ModifierError::MissingSource { .. })
        ));
        assert!(
            oracle
                .query
                .call::<Option<f64>>((
                    db,
                    config,
                    oracle.lua.create_sequence_from(["A"]).unwrap(),
                    "OVERRIDE",
                    warm
                ))
                .is_err()
        );
    }
}

#[test]
fn tagged_input_and_condition_context_fail_closed_without_changing_untagged_api() {
    let entry = TaggedModifierInput {
        modifier: modifier("A", NumericKind::Base, 1.0),
        tags: vec![condition("A", false)],
    };
    let native = ModifierDatabase::try_new_tagged(vec![vec![entry.clone()]]).unwrap();
    assert!(matches!(
        native.sum(SumKind::Base, &QueryContext::default(), &["Missing"]),
        Err(ModifierError::MissingConditionContext)
    ));
    assert!(matches!(
        native.more(&QueryContext::default(), &["A"], &MorePrecision::pinned()),
        Err(ModifierError::MissingConditionContext)
    ));
    assert!(matches!(
        native.override_value(&QueryContext::default(), &["A"]),
        Err(ModifierError::MissingConditionContext)
    ));
    for tag in [
        "EnemyCondition",
        "Multiplier",
        "PerStat",
        "Condition:unknownField",
        "FutureTag",
    ] {
        let mut unknown = entry.clone();
        unknown.tags.push(ModifierTag::Unsupported(tag.to_owned()));
        assert!(matches!(
            ModifierDatabase::try_new_tagged(vec![vec![], vec![unknown]]),
            Err(ModifierError::UnsupportedTag {
                layer: 1,
                modifier: 0,
                ..
            })
        ));
    }
    let mut legacy_tag = entry.clone();
    legacy_tag.modifier.tag_kinds.push("SkillName".to_owned());
    assert!(matches!(
        ModifierDatabase::try_new_tagged(vec![vec![legacy_tag]]),
        Err(ModifierError::UnsupportedTag { .. })
    ));
    let mut flag = entry;
    flag.modifier.kind = ModifierKind::Unsupported("FLAG".to_owned());
    assert!(matches!(
        ModifierDatabase::try_new_tagged(vec![vec![flag]]),
        Err(ModifierError::UnsupportedKind { .. })
    ));
    assert!(ConditionEnvironment::try_new(ConditionEnvironmentInput::default()).is_err());
    let mut invalid = condition_input(1);
    invalid.actors[0].links.insert("enemy".to_owned(), 99);
    assert!(ConditionEnvironment::try_new(invalid).is_err());
    let mut invalid = condition_input(1);
    invalid
        .unsupported_features
        .push("Condition:FLAG".to_owned());
    assert!(ConditionEnvironment::try_new(invalid).is_err());
    let mut invalid = condition_input(1);
    invalid.actors.push(ConditionActor {
        unsupported_features: vec!["Condition:FLAG".to_owned()],
        ..Default::default()
    });
    assert!(ConditionEnvironment::try_new(invalid).is_err());
    let wrong_layers = ConditionEnvironment::try_new(condition_input(2)).unwrap();
    assert!(
        native
            .sum_with_conditions(
                SumKind::Base,
                &QueryContext::default(),
                &["A"],
                &wrong_layers
            )
            .is_err()
    );
    let untagged =
        ModifierDatabase::try_new(vec![vec![modifier("A", NumericKind::Base, 2.0)]]).unwrap();
    assert_eq!(
        untagged
            .sum(SumKind::Base, &QueryContext::default(), &["A"])
            .unwrap(),
        2.0
    );
}

// Multiplier oracle plumbing only serializes inputs; all arithmetic is executed
// by actual source-hashed ModStore:GetMultiplier / ModStore:EvalMod methods above.
use poe_optimizer_engine::multipliers::{
    MultiplierEnvironment, MultiplierError, MultiplierLimit, MultiplierLimitMode, MultiplierScale,
    MultiplierThreshold, MultiplierValues, MultiplierVariables, ScalarSource, ScalingProgram,
    ScalingTag,
};

impl Oracle {
    fn populate_multipliers(&self, root: &Table, layers: &[MultiplierValues]) {
        let mut db = root.clone();
        for (i, layer) in layers.iter().enumerate() {
            let values = self.lua.create_table().unwrap();
            for (name, value) in layer {
                values.set(name.as_str(), *value).unwrap();
            }
            db.set("multipliers", values).unwrap();
            if i + 1 < layers.len() {
                db = db.get("parent").unwrap();
            }
        }
    }

    fn multiplier_query(&self, db: &Table, cfg: &Table, variable: &str) -> mlua::Result<f64> {
        self.lua.load("return function(db, cfg, var, warm) local result; for i = 1, (warm and 200 or 1) do result = db:GetMultiplier(var, cfg) end; return result end")
            .eval::<Function>()?.call((db, cfg, variable, self.warm))
    }

    fn scaling_variables(&self, table: &Table, variables: &MultiplierVariables) {
        match variables {
            MultiplierVariables::One(name) => table.set("var", name.as_str()).unwrap(),
            MultiplierVariables::Sum(names) => table
                .set(
                    "varList",
                    self.lua
                        .create_sequence_from(names.iter().map(String::as_str))
                        .unwrap(),
                )
                .unwrap(),
        }
    }

    fn scalar_fields(&self, table: &Table, source: &ScalarSource, literal: &str, variable: &str) {
        match source {
            ScalarSource::Constant(value) => table.set(literal, *value).unwrap(),
            ScalarSource::Multiplier(name) => table.set(variable, name.as_str()).unwrap(),
        }
    }

    fn stat_variables(&self, table: &Table, stats: &StatVariables) {
        match stats {
            StatVariables::One(name) => table.set("stat", name.as_str()).unwrap(),
            StatVariables::Sum(names) => table
                .set(
                    "statList",
                    self.lua
                        .create_sequence_from(names.iter().map(String::as_str))
                        .unwrap(),
                )
                .unwrap(),
        }
    }

    fn scaling_modifier(&self, value: f64, program: &ScalingProgram) -> Table {
        let modifier = self.lua.create_table().unwrap();
        modifier.set("value", value).unwrap();
        for (i, tag) in program.tags().iter().enumerate() {
            let table = self.lua.create_table().unwrap();
            match tag {
                ScalingTag::Multiplier(tag) => {
                    table.set("type", "Multiplier").unwrap();
                    self.scaling_variables(&table, &tag.variables);
                    self.scalar_fields(&table, &tag.divisor, "div", "divVar");
                    table.set("base", tag.base).unwrap();
                    table.set("invert", tag.invert).unwrap();
                    table.set("actor", tag.actor.as_deref()).unwrap();
                    table.set("limitActor", tag.limit_actor.as_deref()).unwrap();
                    if let Some(limit) = &tag.limit {
                        self.scalar_fields(&table, &limit.value, "limit", "limitVar");
                        table
                            .set(
                                "limitTotal",
                                limit.mode == MultiplierLimitMode::TotalMaximum,
                            )
                            .unwrap();
                        table
                            .set(
                                "limitNegTotal",
                                limit.mode == MultiplierLimitMode::TotalMinimum,
                            )
                            .unwrap();
                    }
                }
                ScalingTag::Threshold(tag) => {
                    table.set("type", "MultiplierThreshold").unwrap();
                    self.scaling_variables(&table, &tag.variables);
                    self.scalar_fields(&table, &tag.threshold, "threshold", "thresholdVar");
                    table.set("upper", tag.upper).unwrap();
                    table.set("equals", tag.equals).unwrap();
                    table.set("actor", tag.actor.as_deref()).unwrap();
                    table
                        .set("thresholdActor", tag.threshold_actor.as_deref())
                        .unwrap();
                }
                ScalingTag::PerStat(tag) => {
                    table.set("type", "PerStat").unwrap();
                    self.stat_variables(&table, &tag.stats);
                    self.scalar_fields(&table, &tag.divisor, "div", "divVar");
                    table.set("base", tag.base).unwrap();
                    table.set("actor", tag.actor.as_deref()).unwrap();
                    if let Some(limit) = &tag.limit {
                        self.scalar_fields(&table, &limit.value, "limit", "limitVar");
                        table.set("limitTotal", limit.total).unwrap();
                    }
                }
                ScalingTag::StatThreshold(tag) => {
                    table.set("type", "StatThreshold").unwrap();
                    self.stat_variables(&table, &tag.stats);
                    match &tag.threshold {
                        StatThresholdValue::Constant(value) => {
                            table.set("threshold", *value).unwrap()
                        }
                        StatThresholdValue::Stat(name) => {
                            table.set("thresholdStat", name.as_str()).unwrap()
                        }
                    }
                    if let Some(percent) = &tag.percent {
                        self.scalar_fields(
                            &table,
                            percent,
                            "thresholdPercent",
                            "thresholdPercentVar",
                        );
                    }
                    table.set("upper", tag.upper).unwrap();
                }
                ScalingTag::Limit { value, negative } => {
                    table.set("type", "Limit").unwrap();
                    self.scalar_fields(&table, value, "limit", "limitVar");
                    table.set("neg", *negative).unwrap();
                }
                ScalingTag::Condition(ModifierTag::Condition { variables, negated }) => {
                    table.set("type", "Condition").unwrap();
                    self.variables(&table, variables);
                    table.set("neg", *negated).unwrap();
                }
                ScalingTag::Condition(ModifierTag::ActorCondition {
                    actor,
                    variables,
                    negated,
                }) => {
                    table.set("type", "ActorCondition").unwrap();
                    table.set("actor", actor.as_deref()).unwrap();
                    if let Some(variables) = variables {
                        self.variables(&table, variables);
                    }
                    table.set("neg", *negated).unwrap();
                }
                ScalingTag::Unsupported(_) | ScalingTag::Condition(ModifierTag::Unsupported(_)) => {
                    panic!("Unsupported oracle input")
                }
            }
            modifier.set(i + 1, table).unwrap();
        }
        modifier
    }

    fn scaling_query(
        &self,
        db: &Table,
        cfg: &Table,
        value: f64,
        program: &ScalingProgram,
    ) -> mlua::Result<Option<f64>> {
        let modifier = self.scaling_modifier(value, program);
        self.lua.load("return function(db, cfg, mod, warm) local result; for i = 1, (warm and 200 or 1) do result = db:EvalMod(mod, cfg) end; return result end")
            .eval::<Function>()?.call((db, cfg, modifier, self.warm))
    }
}

fn multiplier_scale(variable: &str) -> MultiplierScale {
    MultiplierScale {
        variables: MultiplierVariables::One(variable.to_owned()),
        divisor: ScalarSource::Constant(1.0),
        base: 0.0,
        invert: false,
        limit: None,
        actor: None,
        limit_actor: None,
    }
}

#[test]
fn multipliers_preserve_override_base_conditions_and_parent_explicit_values() {
    let mut base = modifier("Multiplier:A", NumericKind::Base, 7.5);
    base.flags = 1;
    let mut overridden = modifier("Multiplier:A", NumericKind::Override, 0.0);
    overridden.keyword_flags = 2;
    let layers = vec![
        vec![TaggedModifierInput {
            modifier: base,
            tags: vec![condition("Active", false)],
        }],
        vec![
            TaggedModifierInput {
                modifier: overridden,
                tags: vec![condition("Override", false)],
            },
            modifier("Multiplier:B", NumericKind::Base, 11.0).into(),
        ],
        vec![
            modifier("Multiplier:A", NumericKind::Base, 13.0).into(),
            modifier("Multiplier:B", NumericKind::Override, -0.0).into(),
        ],
    ];
    let values = vec![
        BTreeMap::from([("A".to_owned(), 2.5), ("B".to_owned(), 1e16)]),
        BTreeMap::from([("A".to_owned(), 3.0), ("B".to_owned(), -1e16)]),
        BTreeMap::from([("A".to_owned(), 4.0), ("B".to_owned(), 1.0)]),
    ];
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for active in [false, true] {
            for override_active in [false, true] {
                let mut input = condition_input(layers.len());
                input.store_conditions[2].insert("Active".to_owned(), active);
                input
                    .skill_conditions
                    .insert("Override".to_owned(), override_active);
                let db = oracle.tagged_database(&layers);
                oracle.populate_multipliers(&db, &values);
                let native = MultiplierEnvironment::try_new(
                    ModifierDatabase::try_new_tagged(layers.clone()).unwrap(),
                    values.clone(),
                    ConditionEnvironment::try_new(input.clone()).unwrap(),
                    vec![],
                )
                .unwrap();
                for flags in [0, 1] {
                    for keyword_flags in [0, 2] {
                        for source in [None, Some("Item"), Some("Item:42"), Some("Tree")] {
                            let query = QueryContext {
                                flags,
                                keyword_flags,
                                source: source.map(str::to_owned),
                            };
                            let cfg = oracle.condition_config(&db, &query, &input);
                            for variable in ["A", "B", "Absent"] {
                                assert_number(
                                    Some(native.get_multiplier(variable, &query).unwrap()),
                                    Some(oracle.multiplier_query(&db, &cfg, variable).unwrap()),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn multiplier_layer_arithmetic_preserves_grouping_nonfinite_and_zero_overrides() {
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for explicit in [
            vec![1e16, -1e16, 1.0],
            vec![1.0, 1e16, -1e16],
            vec![-0.0],
            vec![f64::NAN],
            vec![f64::INFINITY, f64::NEG_INFINITY],
            vec![f64::NEG_INFINITY],
        ] {
            let values: Vec<_> = explicit
                .iter()
                .map(|value| BTreeMap::from([("A".to_owned(), *value)]))
                .collect();
            for override_value in [
                None,
                Some(0.0),
                Some(-0.0),
                Some(f64::NAN),
                Some(f64::INFINITY),
            ] {
                let mut layers = vec![vec![]; values.len()];
                if let Some(value) = override_value {
                    layers[0].push(modifier("Multiplier:A", NumericKind::Override, value).into());
                }
                let input = condition_input(layers.len());
                let db = oracle.tagged_database(&layers);
                oracle.populate_multipliers(&db, &values);
                let query = QueryContext::default();
                let cfg = oracle.condition_config(&db, &query, &input);
                let native = MultiplierEnvironment::try_new(
                    ModifierDatabase::try_new_tagged(layers).unwrap(),
                    values.clone(),
                    ConditionEnvironment::try_new(input).unwrap(),
                    vec![],
                )
                .unwrap();
                assert_number(
                    Some(native.get_multiplier("A", &query).unwrap()),
                    Some(oracle.multiplier_query(&db, &cfg, "A").unwrap()),
                );
            }
        }
    }
}

#[test]
fn multiplier_scaling_matches_floor_inversion_base_and_all_limit_modes() {
    let cases = [
        -3.0001,
        -3.0,
        -0.0002,
        0.0,
        0.9998,
        0.9999,
        0.9999_f64.next_down(),
        0.9999_f64.next_up(),
        3.5,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ];
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for count in cases {
            let values = vec![BTreeMap::from([
                ("Count".to_owned(), count),
                ("Div".to_owned(), 2.0),
                ("Cap".to_owned(), 2.5),
            ])];
            let layers = vec![vec![]];
            let input = condition_input(1);
            let db = oracle.tagged_database(&layers);
            oracle.populate_multipliers(&db, &values);
            let query = QueryContext::default();
            let cfg = oracle.condition_config(&db, &query, &input);
            let native = MultiplierEnvironment::try_new(
                ModifierDatabase::try_new_tagged(layers).unwrap(),
                values,
                ConditionEnvironment::try_new(input).unwrap(),
                vec![],
            )
            .unwrap();
            for divisor in [
                ScalarSource::Constant(1.0),
                ScalarSource::Constant(0.0),
                ScalarSource::Constant(-1.0),
                ScalarSource::Constant(f64::NAN),
                ScalarSource::Multiplier("Div".to_owned()),
            ] {
                for limit_mode in [
                    None,
                    Some(MultiplierLimitMode::FactorMaximum),
                    Some(MultiplierLimitMode::TotalMaximum),
                    Some(MultiplierLimitMode::TotalMinimum),
                ] {
                    for invert in [false, true] {
                        let mut scale = multiplier_scale("Count");
                        scale.divisor = divisor.clone();
                        scale.invert = invert;
                        scale.base = -0.125;
                        scale.limit = limit_mode.map(|mode| MultiplierLimit {
                            value: ScalarSource::Multiplier("Cap".to_owned()),
                            mode,
                        });
                        let program =
                            ScalingProgram::try_new(vec![ScalingTag::Multiplier(scale)]).unwrap();
                        for value in [-2.0, 0.0, 1.25, f64::NAN] {
                            assert_number(
                                program.evaluate(value, &native, &query).unwrap(),
                                oracle.scaling_query(&db, &cfg, value, &program).unwrap(),
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn multiplier_thresholds_lists_and_limits_preserve_exact_comparisons_and_tag_order() {
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for count in [
            -2.0,
            -0.0,
            0.0,
            2.0,
            2.0_f64.next_up(),
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ] {
            let values = vec![BTreeMap::from([
                ("Count".to_owned(), count),
                ("Threshold".to_owned(), 2.0),
                ("Large".to_owned(), 1e16),
                ("Negative".to_owned(), -1e16),
                ("One".to_owned(), 1.0),
            ])];
            let layers = vec![vec![]];
            let input = condition_input(1);
            let db = oracle.tagged_database(&layers);
            oracle.populate_multipliers(&db, &values);
            let query = QueryContext::default();
            let cfg = oracle.condition_config(&db, &query, &input);
            let native = MultiplierEnvironment::try_new(
                ModifierDatabase::try_new_tagged(layers).unwrap(),
                values,
                ConditionEnvironment::try_new(input).unwrap(),
                vec![],
            )
            .unwrap();
            for variables in [
                MultiplierVariables::One("Count".to_owned()),
                MultiplierVariables::Sum(vec![]),
                MultiplierVariables::Sum(vec!["Count".to_owned(), "Count".to_owned()]),
                MultiplierVariables::Sum(vec![
                    "Large".to_owned(),
                    "Negative".to_owned(),
                    "One".to_owned(),
                ]),
                MultiplierVariables::Sum(vec![
                    "One".to_owned(),
                    "Large".to_owned(),
                    "Negative".to_owned(),
                ]),
            ] {
                for threshold in [
                    ScalarSource::Constant(0.0),
                    ScalarSource::Constant(f64::NAN),
                    ScalarSource::Multiplier("Threshold".to_owned()),
                ] {
                    for upper in [false, true] {
                        for equals in [false, true] {
                            let tag = MultiplierThreshold {
                                variables: variables.clone(),
                                threshold: threshold.clone(),
                                upper,
                                equals,
                                actor: None,
                                threshold_actor: None,
                            };
                            let program =
                                ScalingProgram::try_new(vec![ScalingTag::Threshold(tag)]).unwrap();
                            assert_number(
                                program.evaluate(17.0, &native, &query).unwrap(),
                                oracle.scaling_query(&db, &cfg, 17.0, &program).unwrap(),
                            );
                        }
                    }
                }
                let mut scale = multiplier_scale("Count");
                scale.variables = variables;
                let tags = vec![
                    ScalingTag::Multiplier(scale),
                    ScalingTag::Limit {
                        value: ScalarSource::Constant(-0.0),
                        negative: false,
                    },
                    ScalingTag::Limit {
                        value: ScalarSource::Constant(f64::NAN),
                        negative: true,
                    },
                ];
                for tags in [tags.clone(), tags.into_iter().rev().collect()] {
                    let program = ScalingProgram::try_new(tags).unwrap();
                    assert_number(
                        program.evaluate(-0.0, &native, &query).unwrap(),
                        oracle.scaling_query(&db, &cfg, -0.0, &program).unwrap(),
                    );
                }
            }
        }
    }
}

#[test]
fn scaling_condition_order_preserves_early_exit_before_multiplier_source_errors() {
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        let mut no_source = modifier("Multiplier:A", NumericKind::Override, 2.0);
        no_source.source = None;
        let layers = vec![vec![no_source.into()]];
        let values = vec![MultiplierValues::new()];
        let input = condition_input(1);
        let db = oracle.tagged_database(&layers);
        oracle.populate_multipliers(&db, &values);
        let query = QueryContext {
            source: Some("Item".to_owned()),
            ..Default::default()
        };
        let cfg = oracle.condition_config(&db, &query, &input);
        let native = MultiplierEnvironment::try_new(
            ModifierDatabase::try_new_tagged(layers).unwrap(),
            values,
            ConditionEnvironment::try_new(input).unwrap(),
            vec![],
        )
        .unwrap();
        let inactive = ScalingTag::Condition(condition("Missing", false));
        let scale = ScalingTag::Multiplier(multiplier_scale("A"));
        let program = ScalingProgram::try_new(vec![inactive.clone(), scale.clone()]).unwrap();
        assert_number(
            program.evaluate(10.0, &native, &query).unwrap(),
            oracle.scaling_query(&db, &cfg, 10.0, &program).unwrap(),
        );
        let program = ScalingProgram::try_new(vec![scale, inactive]).unwrap();
        assert!(matches!(
            program.evaluate(10.0, &native, &query),
            Err(MultiplierError::Modifier(
                ModifierError::MissingSource { .. }
            ))
        ));
        assert!(oracle.scaling_query(&db, &cfg, 10.0, &program).is_err());
    }
}

#[test]
fn multiplier_context_and_scaling_program_reject_unrepresented_input() {
    let db = ModifierDatabase::try_new(vec![vec![]]).unwrap();
    let conditions = ConditionEnvironment::try_new(condition_input(1)).unwrap();
    assert!(matches!(
        MultiplierEnvironment::try_new(db.clone(), vec![], conditions.clone(), vec![]),
        Err(MultiplierError::LayerCount { .. })
    ));
    assert!(matches!(
        MultiplierEnvironment::try_new(
            db.clone(),
            vec![MultiplierValues::new()],
            conditions.clone(),
            vec!["Recursive multiplier-producing tag".to_owned()]
        ),
        Err(MultiplierError::UnsupportedContext(_))
    ));
    assert!(matches!(
        MultiplierEnvironment::try_new(
            db,
            vec![MultiplierValues::new()],
            ConditionEnvironment::try_new(condition_input(2)).unwrap(),
            vec![]
        ),
        Err(MultiplierError::LayerCount { .. })
    ));
    let mut actor_scale = multiplier_scale("A");
    actor_scale.actor = Some("enemy".to_owned());
    let mut actor_limit = multiplier_scale("A");
    actor_limit.limit_actor = Some("parent".to_owned());
    let mut threshold = MultiplierThreshold {
        variables: MultiplierVariables::One("A".to_owned()),
        threshold: ScalarSource::Constant(1.0),
        upper: false,
        equals: false,
        actor: None,
        threshold_actor: Some("parent".to_owned()),
    };
    for unsupported in [
        ScalingTag::Multiplier(actor_scale),
        ScalingTag::Multiplier(actor_limit),
        ScalingTag::Threshold(threshold.clone()),
        ScalingTag::Unsupported("PerStat".to_owned()),
        ScalingTag::Unsupported("Multiplier extra field".to_owned()),
        ScalingTag::Condition(ModifierTag::Unsupported("FLAG inference".to_owned())),
    ] {
        assert!(matches!(
            ScalingProgram::try_new(vec![
                ScalingTag::Condition(condition("Missing", false)),
                unsupported
            ]),
            Err(MultiplierError::UnsupportedTag { index: 1, .. })
        ));
    }
    threshold.threshold_actor = None;
    threshold.actor = Some("player".to_owned());
    assert!(ScalingProgram::try_new(vec![ScalingTag::Threshold(threshold)]).is_err());
}

#[test]
fn scaling_program_reuse_reads_dynamic_divisors_without_request_order_state() {
    let mut scale = multiplier_scale("Count");
    scale.divisor = ScalarSource::Multiplier("Div".to_owned());
    scale.limit = Some(MultiplierLimit {
        value: ScalarSource::Multiplier("Cap".to_owned()),
        mode: MultiplierLimitMode::TotalMaximum,
    });
    let program = ScalingProgram::try_new(vec![
        ScalingTag::Condition(ModifierTag::ActorCondition {
            actor: None,
            variables: Some(ConditionVariables::One("Active".to_owned())),
            negated: false,
        }),
        ScalingTag::Multiplier(scale),
    ])
    .unwrap();
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        let shared_modifier = oracle.scaling_modifier(2.5, &program);
        let evaluate: Function = oracle.lua.load("return function(db, cfg, mod, warm) local result; for i = 1, (warm and 200 or 1) do result = db:EvalMod(mod, cfg) end; return result end").eval().unwrap();
        // Lua mutates tag.div; native input stays immutable. Returning to each
        // original context must recover the same result after unrelated queries.
        for (count, divisor, cap, active) in [
            (8.0, 2.0, 100.0, true),
            (8.0, 4.0, 100.0, true),
            (8.0, 0.0, 3.0, true),
            (3.0, 1.0, 100.0, false),
            (8.0, 2.0, 100.0, true),
        ] {
            let values = vec![BTreeMap::from([
                ("Count".to_owned(), count),
                ("Div".to_owned(), divisor),
                ("Cap".to_owned(), cap),
            ])];
            let layers = vec![vec![]];
            let mut input = condition_input(1);
            input.store_conditions[0].insert("Active".to_owned(), active);
            let db = oracle.tagged_database(&layers);
            oracle.populate_multipliers(&db, &values);
            let query = QueryContext::default();
            let cfg = oracle.condition_config(&db, &query, &input);
            let native = MultiplierEnvironment::try_new(
                ModifierDatabase::try_new_tagged(layers).unwrap(),
                values,
                ConditionEnvironment::try_new(input).unwrap(),
                vec![],
            )
            .unwrap();
            assert_number(
                program.evaluate(2.5, &native, &query).unwrap(),
                evaluate
                    .call((db, cfg, shared_modifier.clone(), warm))
                    .unwrap(),
            );
        }
    }
}

use poe_optimizer_engine::multipliers::{
    StatLimit, StatScale, StatThreshold, StatThresholdValue, StatVariables,
};
use poe_optimizer_engine::stats::{ResolvedStatEnvironment, StatError, StatValues};

impl Oracle {
    fn populate_stats(&self, db: &Table, cfg: &Table, stats: &ResolvedStatEnvironment) {
        let table = |values: &StatValues| {
            let table = self.lua.create_table().unwrap();
            for (name, value) in values {
                table.set(name.as_str(), *value).unwrap();
            }
            table
        };
        let actor: Table = db.get("actor").unwrap();
        actor.set("output", stats.output().map(table)).unwrap();
        cfg.set("skillStats", table(stats.skill_stats())).unwrap();
    }

    fn stat_query(&self, db: &Table, cfg: &Table, name: &str) -> f64 {
        self.lua.load("return function(db, cfg, name, warm) local result; for i=1,(warm and 200 or 1) do result=db:GetStat(name,cfg) end; return result end").eval::<Function>().unwrap().call((db, cfg, name, self.warm)).unwrap()
    }
}

fn per_stat(name: &str) -> StatScale {
    StatScale {
        stats: StatVariables::One(name.to_owned()),
        divisor: ScalarSource::Constant(1.0),
        base: 0.0,
        limit: None,
        actor: None,
    }
}

#[test]
fn ordinary_stat_lookup_preserves_output_precedence_missing_tables_and_nonfinite_values() {
    let output = BTreeMap::from([
        ("Str".to_owned(), 0.0),
        ("Int".to_owned(), -0.0),
        ("Dex".to_owned(), f64::NAN),
        ("Life".to_owned(), f64::INFINITY),
    ]);
    let skill = BTreeMap::from([
        ("Str".to_owned(), 100.0),
        ("Int".to_owned(), 200.0),
        ("Dex".to_owned(), 300.0),
        ("Life".to_owned(), 400.0),
        ("Mana".to_owned(), -7.0),
    ]);
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        let db = oracle.database(&[vec![]]);
        let cfg = oracle.condition_config(&db, &QueryContext::default(), &condition_input(1));
        for output in [None, Some(StatValues::new()), Some(output.clone())] {
            let stats = ResolvedStatEnvironment::try_new(output, skill.clone(), vec![]).unwrap();
            oracle.populate_stats(&db, &cfg, &stats);
            for name in ["Str", "Int", "Dex", "Life", "Mana", "Absent"] {
                assert_number(
                    Some(stats.get_stat(name).unwrap()),
                    Some(oracle.stat_query(&db, &cfg, name)),
                );
            }
        }
    }
}

#[test]
fn per_stat_scaling_preserves_divisor_epsilon_base_and_factor_or_total_caps() {
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for value in [
            -3.0001,
            -3.0,
            -0.0002,
            -0.0,
            0.9999,
            0.9999_f64.next_down(),
            0.9999_f64.next_up(),
            3.5,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ] {
            let input = condition_input(1);
            let values = vec![BTreeMap::from([
                ("Div".to_owned(), 2.0),
                ("Limit".to_owned(), 2.5),
            ])];
            let db = oracle.tagged_database(&[vec![]]);
            let query = QueryContext::default();
            let cfg = oracle.condition_config(&db, &query, &input);
            oracle.populate_multipliers(&db, &values);
            let environment = MultiplierEnvironment::try_new(
                ModifierDatabase::try_new(vec![vec![]]).unwrap(),
                values,
                ConditionEnvironment::try_new(input).unwrap(),
                vec![],
            )
            .unwrap();
            let stats = ResolvedStatEnvironment::try_new(
                Some(BTreeMap::from([("Str".to_owned(), value)])),
                StatValues::new(),
                vec![],
            )
            .unwrap();
            oracle.populate_stats(&db, &cfg, &stats);
            for divisor in [
                ScalarSource::Constant(1.0),
                ScalarSource::Constant(0.0),
                ScalarSource::Constant(-1.0),
                ScalarSource::Constant(f64::NAN),
                ScalarSource::Multiplier("Div".to_owned()),
            ] {
                for total in [None, Some(false), Some(true)] {
                    let mut tag = per_stat("Str");
                    tag.divisor = divisor.clone();
                    tag.base = -0.125;
                    tag.limit = total.map(|total| StatLimit {
                        value: ScalarSource::Multiplier("Limit".to_owned()),
                        total,
                    });
                    let program = ScalingProgram::try_new(vec![ScalingTag::PerStat(tag)]).unwrap();
                    for initial in [-2.0, -0.0, 1.25, f64::NAN] {
                        assert_number(
                            program
                                .evaluate_with_stats(initial, &environment, &query, &stats)
                                .unwrap(),
                            oracle.scaling_query(&db, &cfg, initial, &program).unwrap(),
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn stat_threshold_preserves_ordered_lists_percent_sources_and_exact_boundaries() {
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for value in [
            -0.0,
            0.0,
            2.0_f64.next_down(),
            2.0,
            2.0_f64.next_up(),
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ] {
            let input = condition_input(1);
            let values = vec![BTreeMap::from([("Percent".to_owned(), 50.0)])];
            let db = oracle.tagged_database(&[vec![]]);
            let query = QueryContext::default();
            let cfg = oracle.condition_config(&db, &query, &input);
            oracle.populate_multipliers(&db, &values);
            let environment = MultiplierEnvironment::try_new(
                ModifierDatabase::try_new(vec![vec![]]).unwrap(),
                values,
                ConditionEnvironment::try_new(input).unwrap(),
                vec![],
            )
            .unwrap();
            let stats = ResolvedStatEnvironment::try_new(
                Some(BTreeMap::from([
                    ("Str".to_owned(), value),
                    ("Threshold".to_owned(), 2.0),
                    ("Large".to_owned(), 1e16),
                    ("Negative".to_owned(), -1e16),
                    ("One".to_owned(), 1.0),
                ])),
                StatValues::new(),
                vec![],
            )
            .unwrap();
            oracle.populate_stats(&db, &cfg, &stats);
            for names in [
                StatVariables::One("Str".to_owned()),
                StatVariables::Sum(vec![]),
                StatVariables::Sum(vec!["Str".to_owned(), "Str".to_owned()]),
                StatVariables::Sum(vec![
                    "Large".to_owned(),
                    "Negative".to_owned(),
                    "One".to_owned(),
                ]),
                StatVariables::Sum(vec![
                    "One".to_owned(),
                    "Large".to_owned(),
                    "Negative".to_owned(),
                ]),
            ] {
                for threshold in [
                    StatThresholdValue::Constant(0.0),
                    StatThresholdValue::Constant(f64::NAN),
                    StatThresholdValue::Stat("Threshold".to_owned()),
                    StatThresholdValue::Stat("Missing".to_owned()),
                ] {
                    for percent in [
                        None,
                        Some(ScalarSource::Constant(0.0)),
                        Some(ScalarSource::Constant(100.0)),
                        Some(ScalarSource::Constant(f64::NAN)),
                        Some(ScalarSource::Multiplier("Percent".to_owned())),
                    ] {
                        for upper in [false, true] {
                            let program = ScalingProgram::try_new(vec![ScalingTag::StatThreshold(
                                StatThreshold {
                                    stats: names.clone(),
                                    threshold: threshold.clone(),
                                    percent: percent.clone(),
                                    upper,
                                },
                            )])
                            .unwrap();
                            assert_number(
                                program
                                    .evaluate_with_stats(7.0, &environment, &query, &stats)
                                    .unwrap(),
                                oracle.scaling_query(&db, &cfg, 7.0, &program).unwrap(),
                            );
                        }
                    }
                }
                let mut tag = per_stat("Str");
                tag.stats = names;
                let program = ScalingProgram::try_new(vec![ScalingTag::PerStat(tag)]).unwrap();
                assert_number(
                    program
                        .evaluate_with_stats(-0.0, &environment, &query, &stats)
                        .unwrap(),
                    oracle.scaling_query(&db, &cfg, -0.0, &program).unwrap(),
                );
            }
        }
    }
}

#[test]
fn stat_programs_preserve_mixed_tag_order_and_shared_program_request_independence() {
    let mut tag = per_stat("Str");
    tag.divisor = ScalarSource::Multiplier("Div".to_owned());
    tag.base = 0.75;
    let tags = vec![
        ScalingTag::Condition(condition("Active", false)),
        ScalingTag::PerStat(tag),
        ScalingTag::Multiplier(multiplier_scale("Count")),
        ScalingTag::Limit {
            value: ScalarSource::Constant(10.0),
            negative: false,
        },
    ];
    for tags in [tags.clone(), tags.into_iter().rev().collect()] {
        let program = ScalingProgram::try_new(tags).unwrap();
        for warm in [false, true] {
            let oracle = Oracle::new(warm);
            let shared_modifier = oracle.scaling_modifier(2.5, &program);
            let evaluate: Function = oracle.lua.load("return function(db, cfg, mod, warm) local result; for i=1,(warm and 200 or 1) do result=db:EvalMod(mod,cfg) end; return result end").eval().unwrap();
            for (str_value, divisor, active) in [
                (8.0, 2.0, true),
                (13.0, 4.0, true),
                (f64::NAN, 0.0, true),
                (2.0, 3.0, false),
                (8.0, 2.0, true),
            ] {
                let mut input = condition_input(1);
                input.store_conditions[0].insert("Active".to_owned(), active);
                let values = vec![BTreeMap::from([
                    ("Count".to_owned(), 2.0),
                    ("Div".to_owned(), divisor),
                ])];
                let db = oracle.tagged_database(&[vec![]]);
                let query = QueryContext::default();
                let cfg = oracle.condition_config(&db, &query, &input);
                oracle.populate_multipliers(&db, &values);
                let environment = MultiplierEnvironment::try_new(
                    ModifierDatabase::try_new(vec![vec![]]).unwrap(),
                    values,
                    ConditionEnvironment::try_new(input).unwrap(),
                    vec![],
                )
                .unwrap();
                let stats = ResolvedStatEnvironment::try_new(
                    Some(BTreeMap::from([("Str".to_owned(), str_value)])),
                    StatValues::new(),
                    vec![],
                )
                .unwrap();
                oracle.populate_stats(&db, &cfg, &stats);
                assert_number(
                    program
                        .evaluate_with_stats(2.5, &environment, &query, &stats)
                        .unwrap(),
                    evaluate
                        .call((db, cfg, shared_modifier.clone(), warm))
                        .unwrap(),
                );
            }
        }
    }
}

#[test]
fn stat_programs_reject_special_stat_branches_unknown_context_and_missing_context() {
    let stats = ResolvedStatEnvironment::try_new(
        Some(BTreeMap::from([("ManaUnreserved".to_owned(), 5.0)])),
        StatValues::new(),
        vec![],
    )
    .unwrap();
    for name in [
        "ManaReservedPercent",
        "LifeReservedPercent",
        "ManaUnreserved",
    ] {
        assert!(matches!(
            stats.get_stat(name),
            Err(StatError::UnsupportedStat(_))
        ));
        assert!(
            ScalingProgram::try_new(vec![
                ScalingTag::Condition(condition("Missing", false)),
                ScalingTag::PerStat(per_stat(name))
            ])
            .is_err()
        );
        assert!(
            ScalingProgram::try_new(vec![ScalingTag::StatThreshold(StatThreshold {
                stats: StatVariables::One("Str".to_owned()),
                threshold: StatThresholdValue::Stat(name.to_owned()),
                percent: None,
                upper: false
            })])
            .is_err()
        );
    }
    assert!(matches!(
        ResolvedStatEnvironment::try_new(
            None,
            StatValues::new(),
            vec!["Function-valued output".to_owned()]
        ),
        Err(StatError::UnsupportedContext(_))
    ));
    let mut tag = per_stat("Str");
    tag.actor = Some("parent".to_owned());
    assert!(ScalingProgram::try_new(vec![ScalingTag::PerStat(tag)]).is_err());
    let environment = MultiplierEnvironment::try_new(
        ModifierDatabase::try_new(vec![vec![]]).unwrap(),
        vec![MultiplierValues::new()],
        ConditionEnvironment::try_new(condition_input(1)).unwrap(),
        vec![],
    )
    .unwrap();
    let program = ScalingProgram::try_new(vec![
        ScalingTag::Condition(condition("Missing", false)),
        ScalingTag::PerStat(per_stat("Str")),
    ])
    .unwrap();
    assert_eq!(
        program.evaluate(1.0, &environment, &QueryContext::default()),
        Err(MultiplierError::MissingStatContext)
    );
    assert!(
        program
            .evaluate_with_stats(1.0, &environment, &QueryContext::default(), &stats)
            .unwrap()
            .is_none()
    );
}

use poe_optimizer_engine::spark::{self, SparkInput, SparkQuestRewards};
const SPARK_SKILLS: &str =
    include_str!("../../../vendor/path-of-building-poe2/src/Data/Skills/act_int.lua");
const SPARK_MISC: &str = include_str!("../../../vendor/path-of-building-poe2/src/Data/Misc.lua");
const SPARK_QUESTS: &str =
    include_str!("../../../vendor/path-of-building-poe2/src/Data/QuestRewards.lua");
const SPARK_TREE: &str =
    include_str!("../../../vendor/path-of-building-poe2/src/TreeData/0_5/tree.lua");
const SPARK_SETUP: &str =
    include_str!("../../../vendor/path-of-building-poe2/src/Modules/CalcSetup.lua");
const SPARK_PERFORM: &str =
    include_str!("../../../vendor/path-of-building-poe2/src/Modules/CalcPerform.lua");
const SPARK_OFFENCE: &str =
    include_str!("../../../vendor/path-of-building-poe2/src/Modules/CalcOffence.lua");
const SPARK_DEFENCE: &str =
    include_str!("../../../vendor/path-of-building-poe2/src/Modules/CalcDefence.lua");
const SPARK_CONFIG: &str =
    include_str!("../../../vendor/path-of-building-poe2/src/Modules/ConfigOptions.lua");
const SPARK_TOOLS: &str =
    include_str!("../../../vendor/path-of-building-poe2/src/Modules/CalcTools.lua");

fn source_line<'a>(source: &'a str, exact_prefix: &str) -> &'a str {
    let mut matching = source
        .lines()
        .filter(|line| line.trim_start().starts_with(exact_prefix));
    let result = matching
        .next()
        .unwrap_or_else(|| panic!("Missing upstream source line: {exact_prefix}"));
    assert!(
        matching.next().is_none(),
        "Ambiguous upstream source line: {exact_prefix}"
    );
    result
}

fn spark_source_checks() {
    for record in spark::SOURCE_FILES {
        let source = match record.path {
            "src/Modules/ModParser.lua" => character_parity::PARSER,
            "src/Data/Misc.lua" => SPARK_MISC,
            "src/Data/QuestRewards.lua" => SPARK_QUESTS,
            "src/Data/Skills/act_int.lua" => SPARK_SKILLS,
            "src/TreeData/0_5/tree.lua" => SPARK_TREE,
            "src/Modules/CalcSetup.lua" => SPARK_SETUP,
            "src/Modules/CalcPerform.lua" => SPARK_PERFORM,
            "src/Modules/CalcOffence.lua" => SPARK_OFFENCE,
            "src/Modules/CalcDefence.lua" => SPARK_DEFENCE,
            "src/Modules/ConfigOptions.lua" => SPARK_CONFIG,
            "src/Modules/Data.lua" => DATA,
            "src/Modules/Common.lua" => COMMON,
            path => panic!("Unverified native Spark source: {path}"),
        };
        assert_eq!(
            format!("{:x}", Sha256::digest(source.replace("\r\n", "\n"))),
            record.sha256,
            "{} changed",
            record.path
        );
    }
    assert_eq!(
        format!("{:x}", Sha256::digest(SPARK_TOOLS.replace("\r\n", "\n"))),
        "83bdbea3790a49ef05fe1050acf1d489cf8cac90865ba31f7bc4a7a5abefcd2c"
    );
}

struct SparkOracle {
    oracle: Oracle,
    calculate: Function,
}
impl SparkOracle {
    fn new(warm: bool) -> Self {
        spark_source_checks();
        let oracle = Oracle::new(warm);
        let lua = &oracle.lua;
        let data: Table = lua
            .load(SPARK_MISC)
            .set_name("pinned-Spark-Misc-data")
            .eval()
            .unwrap();
        lua.globals().set("data", data.clone()).unwrap();
        let data_source = DATA.replace("\r\n", "\n");
        lua.load(section(
            &data_source,
            "data.misc = {",
            "\ndata.skillColorMap = ",
        ))
        .exec()
        .unwrap();
        lua.load(section(
            &data_source,
            "data.highPrecisionMods = {",
            "data.weaponTypeInfo = {",
        ))
        .exec()
        .unwrap();
        lua.load(section(
            SPARK_TOOLS,
            "calcLib = { }",
            "-- Validate the level of the given gem",
        ))
        .exec()
        .unwrap();
        let tree: Table = lua
            .load(SPARK_TREE)
            .set_name("pinned-Spark-tree-data")
            .eval()
            .unwrap();
        let classes: Table = tree.get("classes").unwrap();
        let class: Table = classes.get(spark::CLASS_ID).unwrap();
        lua.globals().set("sparkClass", class).unwrap();
        lua.load(
            "skills = {}; SkillType = setmetatable({}, {__index=function(_, key) return key end})",
        )
        .exec()
        .unwrap();
        let skill_source = SPARK_SKILLS.replace("\r\n", "\n");
        lua.load(section(
            &skill_source,
            "skills[\"SparkPlayer\"] = {",
            "\nskills[\"SummonSpectrePlayer\"]",
        ))
        .set_name("pinned-Spark-skill-data")
        .exec()
        .unwrap();
        let skills: Table = lua.globals().get("skills").unwrap();
        lua.globals()
            .set("sparkSkill", skills.get::<Table>(spark::SKILL_ID).unwrap())
            .unwrap();
        lua.load("package.loaded['Modules.CalcBase'] = {};")
            .exec()
            .unwrap();
        let calcs: Table = lua
            .load(SPARK_DEFENCE)
            .set_name("pinned-Spark-resource-function")
            .eval()
            .unwrap();
        lua.globals().set("sparkCalcs", calcs).unwrap();
        let quests: Table = lua
            .load(SPARK_QUESTS)
            .set_name("pinned-Spark-quest-data")
            .eval()
            .unwrap();
        let selected_quests = lua.create_table().unwrap();
        // Source-text classification is fixture setup, not a numeric formula oracle.
        for item in quests.sequence_values::<Table>() {
            let quest = item.unwrap();
            let info: String = quest.get("Info").unwrap();
            let field = match info.as_str() {
                "Candlemass" => Some("candlemass"),
                "Molten Shrine" => Some("molten_shrine"),
                "Silent Hall" => Some("silent_hall"),
                "Beira" => Some("beira"),
                "Sisters of Garukhan Shrine" => Some("garukhan"),
                "Blackjaw" => Some("blackjaw"),
                _ => None,
            };
            if let Some(field) = field {
                let text: String = quest.get("Stat").unwrap();
                let number: f64 = text
                    .split_whitespace()
                    .next()
                    .unwrap()
                    .trim_start_matches('+')
                    .trim_end_matches('%')
                    .parse()
                    .unwrap();
                let (name, kind) = match field {
                    "candlemass" => ("Life", "BASE"),
                    "molten_shrine" => ("Life", "INC"),
                    "silent_hall" => ("Mana", "INC"),
                    "beira" => ("ColdResist", "BASE"),
                    "garukhan" => ("LightningResist", "BASE"),
                    "blackjaw" => ("FireResist", "BASE"),
                    _ => unreachable!(),
                };
                let record = lua.create_table().unwrap();
                record.set("name", name).unwrap();
                record.set("kind", kind).unwrap();
                record.set("value", number).unwrap();
                selected_quests.set(field, record).unwrap();
            }
        }
        lua.globals().set("sparkQuests", selected_quests).unwrap();
        character_parity::install_defence_oracle(lua);
        resistance_parity::install_defence_oracle(lua);
        // Insert unchanged upstream functions/expressions for the exact branches
        // admitted by this profile. Scaffolding supplies resolved skill/context data;
        // no expected values or rewritten Lua arithmetic are used.
        let setup = SPARK_SETUP.replace("\r\n", "\n");
        let perform = SPARK_PERFORM.replace("\r\n", "\n");
        let offence = SPARK_OFFENCE.replace("\r\n", "\n");
        let mut body = section(
            &offence,
            "-- Path of Building",
            "---Calculates the area percentage",
        )
        .to_owned();
        body.push_str("return function(input) local m_min, m_max = math.min, math.max; local modDB=new('ModDB'):ModDB(); local output={Str=input.character.strength,Dex=input.character.dexterity,Int=input.character.intelligence}; modDB.actor={output=output}; modDB.multipliers.Level=input.level; for _, mod in ipairs(input.character.mods) do modDB:AddMod(copyTable(mod)) end; ");
        body.push_str(section(
            &setup,
            "\t\tmodDB:NewMod(\"Life\", \"BASE\", data.characterConstants",
            "\t\tmodDB:NewMod(\"ManaRegen\"",
        ));
        body.push_str("local env={configInput={resistancePenalty=input.penalty}}; ");
        body.push_str(source_line(&setup, "modDB:NewMod(\"CritChanceCap\","));
        body.push_str("\nif input.critical_cap then modDB:NewMod('CritChanceCap','OVERRIDE',input.critical_cap,'Explicit test data') end; ");
        for prefix in resistance_parity::SETUP_PREFIXES {
            body.push_str(source_line(&setup, prefix));
            body.push('\n');
        }
        body.push_str("for field, quest in pairs(sparkQuests) do if input.quests[field] then modDB:NewMod(quest.name,quest.kind,quest.value,'Quest') end end\n");
        body.push_str(section(
            &perform,
            "\t-- Add attribute bonuses\n",
            "\t-- Calculate Presence / Surrounded",
        ));
        body.push_str("sparkCalcs.doActorLifeManaSpirit({modDB=modDB,output=output},true); characterDefences(modDB,output,input.level); playerResistances(modDB,output); local enemyDB=new('ModDB'):ModDB(); enemyDB:NewMod('LightningResist','BASE',input.resistance,'Config'); local env={configInput={enemyLightningResist=input.resistance},modDB=modDB,partyMembers={modDB=modDB},mode_effective=true}; local isElemental={Lightning=true}; ");
        body.push_str(section(
            &offence,
            "\tlocal function calcResistForType(",
            "\n\tlocal function runSkillFunc(",
        ));
        body.push_str("local cfg={flags=OR64(ModFlag.Spell,ModFlag.Cast,ModFlag.Projectile,ModFlag.Hit)}; local skillCfg=cfg; local skillModList=modDB; local skillData={}; local activeSkill={skillModList=modDB,conversionTable={},activeEffect={grantedEffect=sparkSkill}}; local globalOutput={ActionSpeedMod=1}; local baseCrit=input.critical_chance or sparkSkill.levels[1].critChance; local base,inc,more=0,0,1;\n");
        body.push_str(source_line(
            &offence,
            "output.CritChance = round((baseCrit + base)",
        ));
        body.push('\n');
        body.push_str(source_line(
            &offence,
            "output.CritChance = m_min(output.CritChance, skillModList:Override",
        ));
        body.push('\n');
        body.push_str(source_line(
            &offence,
            "output.CritChance = m_max(output.CritChance, 0)",
        ));
        body.push_str("\nmodDB:NewMod('CritMultiplier','BASE',data.characterConstants.base_critical_hit_damage_bonus,'Base');\n");
        let crit = section(
            &offence,
            "\t\t\t\tlocal extraDamage = skillModList:Sum(\"BASE\", cfg, \"CritMultiplier\")",
            "\n\t\t\t\toutput.CritMultiplier = 1 + m_max(0, extraDamage)",
        );
        body.push_str(crit);
        body.push_str(source_line(
            &offence,
            "output.CritMultiplier = 1 + m_max(0, extraDamage)",
        ));
        body.push_str("\nlocal baseTime; ");
        body.push_str(source_line(&offence, "baseTime = (skillData.castTimeOverride or activeSkill.activeEffect.grantedEffect.castTime"));
        body.push('\n');
        body.push_str(source_line(
            section(&offence, "\t\t\tif skillModList:Sum(\"BASE\", skillCfg, \"Multiplier:TraumaStacks\") == 0 then", "\n\t\t\tif skillFlags.warcry then"),
            "local inc = skillModList:Sum(\"INC\", cfg, \"Speed\")",
        ));
        body.push('\n');
        body.push_str(source_line(
            &offence,
            "output.Speed = 1 / (baseTime / round(",
        ));
        body.push_str("\nglobalOutput.Speed=output.Speed; local effectiveResist=calcResistForType('Lightning',cfg); local totalHitAvg,totalCritAvg; for pass=1,2 do output.LightningSummedMinBase=sparkSkill.statSets[1].levels[1][1]; output.LightningSummedMaxBase=sparkSkill.statSets[1].levels[1][2]; local damageTypeHitMin,damageTypeHitMax=calcDamage(activeSkill,output,cfg,nil,'Lightning',0); local allMult=1; ");
        body.push_str(section(
            &offence,
            "\t\t\t\t\tif pass == 1 then\n\t\t\t\t\t\t-- Apply crit multiplier",
            "\n\t\t\t\t\tif skillModList:Flag(skillCfg, \"LuckyHits\")",
        ));
        body.push_str("\nlocal damageTypeHitAvgNotLucky,damageTypeHitAvgLucky,damageTypeHitAvg; local damageTypeLuckyChance=0; ");
        body.push_str(source_line(
            &offence,
            "damageTypeHitAvgNotLucky = (damageTypeHitMin / 2",
        ));
        body.push('\n');
        body.push_str(source_line(
            &offence,
            "damageTypeHitAvgLucky = (damageTypeHitMin / 3",
        ));
        body.push('\n');
        body.push_str(source_line(
            &offence,
            "damageTypeHitAvg = damageTypeHitAvgNotLucky *",
        ));
        body.push_str("\nlocal effMult=1; ");
        body.push_str(source_line(
            &offence,
            "effMult = effMult * (1 - effectiveResist / 100)",
        ));
        body.push('\n');
        body.push_str(source_line(
            &offence,
            "damageTypeHitAvg = damageTypeHitAvg * effMult",
        ));
        body.push_str("\nif pass==1 then totalCritAvg=damageTypeHitAvg else totalHitAvg=damageTypeHitAvg end; end; output.HitChance=100; output.DpsMultiplier=1; local quantityMultiplier=1; ");
        body.push_str(source_line(&offence, "output.AverageHit = totalHitAvg *"));
        body.push('\n');
        body.push_str(source_line(
            &offence,
            "output.AverageDamage = output.AverageHit * output.HitChance / 100",
        ));
        body.push('\n');
        body.push_str(source_line(
            &offence,
            "output.TotalDPS = output.AverageDamage *",
        ));
        body.push_str("\noutput.EffectiveResist=effectiveResist; return output end");
        let calculate = lua
            .load(body)
            .set_name("pinned-Spark-closed-pipeline-sections")
            .eval()
            .unwrap();
        Self { oracle, calculate }
    }

    fn calculate(&self, input: &SparkInput) -> Table {
        self.calculate_with_character(input, &spark::default_character())
    }

    fn calculate_with_character(
        &self,
        input: &SparkInput,
        character: &poe_optimizer_engine::character::CharacterInput,
    ) -> Table {
        self.calculate_with_critical_data(input, character, None)
    }
    fn calculate_with_critical_data(
        &self,
        input: &SparkInput,
        character: &poe_optimizer_engine::character::CharacterInput,
        critical_data: Option<(f64, f64)>,
    ) -> Table {
        let lua = &self.oracle.lua;
        let table = lua.create_table().unwrap();
        if let Some((chance, cap)) = critical_data {
            table.set("critical_chance", chance).unwrap();
            table.set("critical_cap", cap).unwrap();
        }
        table.set("level", input.character_level).unwrap();
        table.set("penalty", input.resistance_penalty).unwrap();
        table
            .set("character", character_parity::input_table(lua, character))
            .unwrap();
        table
            .set("resistance", input.enemy_lightning_resistance)
            .unwrap();
        let quests = lua.create_table().unwrap();
        for (name, enabled) in [
            ("candlemass", input.quests.candlemass),
            ("molten_shrine", input.quests.molten_shrine),
            ("silent_hall", input.quests.silent_hall),
            ("beira", input.quests.beira),
            ("garukhan", input.quests.garukhan),
            ("blackjaw", input.quests.blackjaw),
        ] {
            quests.set(name, enabled).unwrap();
        }
        table.set("quests", quests).unwrap();
        let wrapper:Function=lua.load("return function(f,input,warm) local result; for i=1,(warm and 200 or 1) do result=f(input) end; return result end").eval().unwrap();
        wrapper
            .call((self.calculate.clone(), table, self.oracle.warm))
            .unwrap()
    }
}

#[test]
fn closed_spark_pipeline_matches_pinned_resource_and_offense_source_sections() {
    for warm in [false, true] {
        let oracle = SparkOracle::new(warm);
        for level in [1, 2, 20, 60, 61, 100] {
            for mask in [0u8, 1, 2, 4, 8, 16, 32, 63] {
                let quests = SparkQuestRewards {
                    candlemass: mask & 1 != 0,
                    molten_shrine: mask & 2 != 0,
                    silent_hall: mask & 4 != 0,
                    beira: mask & 8 != 0,
                    garukhan: mask & 16 != 0,
                    blackjaw: mask & 32 != 0,
                };
                for resistance in [-200.0, -25.5, 0.0, 50.0, 80.0, 90.0, 200.0] {
                    let input = SparkInput {
                        character_level: level,
                        resistance_penalty: -60.0,
                        enemy_lightning_resistance: resistance,
                        quests,
                    };
                    let actual = spark::evaluate(&input).unwrap();
                    let expected = oracle.calculate(&input);
                    for (name, actual) in [
                        ("Life", actual.life),
                        ("Mana", actual.mana),
                        ("Str", actual.strength),
                        ("Dex", actual.dexterity),
                        ("Int", actual.intelligence),
                        ("AverageHit", actual.average_hit),
                        ("TotalDPS", actual.hit_dps),
                        ("Speed", actual.cast_rate),
                        ("CritChance", actual.crit_chance),
                        ("CritMultiplier", actual.crit_multiplier),
                        (
                            "EffectiveResist",
                            actual.effective_enemy_lightning_resistance,
                        ),
                    ] {
                        assert_number(Some(actual), Some(expected.get(name).unwrap()));
                    }
                }
            }
        }
    }
}

#[test]
fn closed_spark_data_is_transcribed_from_source_and_rejects_invalid_profile_inputs() {
    let oracle = SparkOracle::new(false);
    let class: Table = oracle.oracle.lua.globals().get("sparkClass").unwrap();
    assert_eq!(
        class.get::<f64>("base_str").unwrap(),
        spark::data().strength
    );
    assert_eq!(
        class.get::<f64>("base_dex").unwrap(),
        spark::data().dexterity
    );
    assert_eq!(
        class.get::<f64>("base_int").unwrap(),
        spark::data().intelligence
    );
    let skill: Table = oracle.oracle.lua.globals().get("sparkSkill").unwrap();
    assert_eq!(
        skill.get::<f64>("castTime").unwrap(),
        spark::data().cast_time
    );
    let data: Table = oracle.oracle.lua.globals().get("data").unwrap();
    let constants: Table = data.get("characterConstants").unwrap();
    for (name, value) in [
        ("life_per_level", spark::data().life_per_level),
        ("mana_per_level", spark::data().mana_per_level),
        (
            "base_critical_hit_damage_bonus",
            spark::data().critical_damage_bonus,
        ),
        (
            "base_maximum_all_resistances_%",
            spark::data().player_resistance_cap,
        ),
    ] {
        assert_eq!(constants.get::<f64>(name).unwrap(), value);
    }
    let input = SparkInput {
        character_level: 60,
        resistance_penalty: -60.0,
        enemy_lightning_resistance: 0.0,
        quests: SparkQuestRewards::default(),
    };
    for level in [0, 101, u32::MAX] {
        assert!(
            spark::evaluate(&SparkInput {
                character_level: level,
                ..input
            })
            .is_err()
        );
    }
    for resistance in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -200.1, 200.1] {
        assert!(
            spark::evaluate(&SparkInput {
                enemy_lightning_resistance: resistance,
                ..input
            })
            .is_err()
        );
    }
    for penalty in [f64::NAN, f64::INFINITY, -60.1, 0.1] {
        assert!(
            spark::evaluate(&SparkInput {
                resistance_penalty: penalty,
                ..input
            })
            .is_err()
        );
    }
    let before = spark::evaluate(&input).unwrap();
    spark::evaluate(&SparkInput {
        character_level: 1,
        enemy_lightning_resistance: 200.0,
        ..input
    })
    .unwrap();
    assert_eq!(spark::evaluate(&input).unwrap(), before);
}

#[path = "support/mace_parity.rs"]
mod mace_parity;

#[path = "support/character_parity.rs"]
mod character_parity;

#[path = "support/resistance_parity.rs"]
mod resistance_parity;

#[path = "support/weapon_parity.rs"]
mod weapon_parity;
