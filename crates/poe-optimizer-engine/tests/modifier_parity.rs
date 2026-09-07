//! Oracle executes pinned ModStore/ModDB methods and their actual upstream helpers.
#![cfg(not(target_arch = "wasm32"))]

use std::collections::BTreeMap;

use mlua::{Function, Lua, Table};
use poe_optimizer_engine::modifiers::{
    KEYWORD_MATCH_ALL, ModifierDatabase, ModifierError, ModifierInput, ModifierKind, ModifierValue,
    MorePrecision, NumericKind, QueryContext, SUPPORTED_MOD_FLAG_BITS, SumKind,
};
use sha2::{Digest, Sha256};

const DB: &str = include_str!("../../../vendor/path-of-building-poe2/src/Classes/ModDB.lua");
const STORE: &str = include_str!("../../../vendor/path-of-building-poe2/src/Classes/ModStore.lua");
const GLOBAL: &str = include_str!("../../../vendor/path-of-building-poe2/src/Data/Global.lua");
const COMMON: &str = include_str!("../../../vendor/path-of-building-poe2/src/Modules/Common.lua");
const DATA: &str = include_str!("../../../vendor/path-of-building-poe2/src/Modules/Data.lua");

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
