//! Independent original ConfigOptions construction and ConfigTab initial-state semantics.
#![cfg(not(target_arch = "wasm32"))]
use mlua::{Function, Lua, Table, Value};
use poe_optimizer_core::options::Scalar;
use poe_optimizer_data::{configuration::*, game_data::bundled_snapshot};
use poe_optimizer_pob::source;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::OnceLock,
};

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn sources() -> &'static BTreeMap<&'static str, String> {
    static SOURCES: OnceLock<BTreeMap<&'static str, String>> = OnceLock::new();
    SOURCES.get_or_init(|| {
        [
            "src/Modules/Common.lua",
            "src/Data/Global.lua",
            "src/Data/Misc.lua",
            "src/Data/QuestRewards.lua",
            "src/Data/Bosses.lua",
            "src/Data/BossSkills.lua",
            "src/Modules/Data.lua",
            "src/Modules/ConfigOptions.lua",
            "src/Classes/ConfigTab.lua",
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
    })
}
fn original(path: &str) -> String {
    sources()[path].clone()
}

fn section<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
    assert_eq!(
        text.matches(start).count(),
        1,
        "unique original source anchor {start}"
    );
    let begin = text.find(start).unwrap();
    &text[begin..begin + text[begin..].find(end).unwrap()]
}

struct Original {
    lua: Lua,
    rows: Table,
    defaults: Function,
    rounds: usize,
}
impl Original {
    fn new(warm: bool) -> Self {
        let lua = Lua::new();
        lua.load(if warm { "jit.on()" } else { "jit.off()" })
            .exec()
            .unwrap();
        let common = original("src/Modules/Common.lua");
        lua.load(section(
            &common,
            "function copyTable(tbl, noRecurse)",
            "\ndo\n",
        ))
        .set_name("@src/Modules/Common.lua")
        .exec()
        .unwrap();
        lua.load(original("src/Data/Global.lua"))
            .set_name("@src/Data/Global.lua")
            .exec()
            .unwrap();
        let data: Table = lua
            .load(original("src/Data/Misc.lua"))
            .set_name("@src/Data/Misc.lua")
            .eval()
            .unwrap();
        let quests: Table = lua
            .load(original("src/Data/QuestRewards.lua"))
            .set_name("@src/Data/QuestRewards.lua")
            .eval()
            .unwrap();
        data.set("questRewards", quests).unwrap();
        lua.globals().set("data", data).unwrap();
        // Execute the complete original boss metadata producer, including its tooltip.
        // LoadModule can access only these two authenticated, data-only dependencies.
        lua.globals()
            .set(
                "LoadModule",
                lua.create_function(|lua, name: String| {
                    assert!(
                        ["Data/Bosses", "Data/BossSkills"].contains(&name.as_str()),
                        "unexpected source dependency {name}"
                    );
                    let path = format!("src/{name}.lua");
                    lua.load(original(&path))
                        .set_name(format!("@{path}"))
                        .eval::<Table>()
                })
                .unwrap(),
            )
            .unwrap();
        let module = original("src/Modules/Data.lua");
        lua.load(section(
            &module,
            "data.misc = { -- magic numbers",
            "data.skillColorMap =",
        ))
        .set_name("@src/Modules/Data.lua")
        .exec()
        .unwrap();
        lua.load(format!(
            "local m_floor=math.floor;{}",
            section(&module, "-- Load bosses", "-- Load skills")
        ))
        .set_name("@src/Modules/Data.lua")
        .exec()
        .unwrap();
        let construct: Function = lua
            .load(original("src/Modules/ConfigOptions.lua"))
            .set_name("@src/Modules/ConfigOptions.lua")
            .into_function()
            .unwrap();
        let mut rows: Table = construct.call(()).unwrap();
        if warm {
            for _ in 1..100 {
                rows = construct.call(()).unwrap();
            }
        }
        let tab = original("src/Classes/ConfigTab.lua");
        // varList is supplied to the original CreateConfigSet, preserving ordered
        // duplicate writes and nil removals. No ConfigOptions callback is invoked.
        let defaults:Function=lua.load(format!(r#"
return function(varList,rounds,placeholders,uiDefaults)
 local ConfigTabClass={{}}
 {}
 {}
 local tab;local state
 for i=1,rounds do
  tab=setmetatable({{configSets={{}},activeConfigSetId=17,defaultState=uiDefaults or{{}}}},{{__index=ConfigTabClass}})
  state=tab:CreateConfigSet(17,'Independent original source')
 end
 if placeholders then state.placeholder=placeholders end
 return state, function(key,kind)return tab:GetDefaultState(key,kind)end
end
"#,section(&tab,"function ConfigTabClass:CreateConfigSet(configSetId, title)","-- Creates a new config set, adds it"),section(&tab,"function ConfigTabClass:GetDefaultState(var, varType)","function ConfigTabClass:Save(xml)"))).set_name("@independent-configtab-consumers").eval().unwrap();
        Self {
            lua,
            rows,
            defaults,
            rounds: if warm { 100 } else { 1 },
        }
    }
    fn initial(&self, rows: &Table) -> (Table, Function) {
        self.defaults.call((rows, self.rounds)).unwrap()
    }
}
fn scalar(value: Value) -> Scalar {
    match value {
        Value::Boolean(v) => Scalar::Boolean(v),
        Value::Number(v) => Scalar::Number(v),
        Value::Integer(v) => Scalar::Number(v as f64),
        Value::String(v) => Scalar::Text(v.to_str().unwrap().to_owned()),
        other => panic!("unexpected original scalar {other:?}"),
    }
}
fn scalar_map(table: Table) -> BTreeMap<String, Scalar> {
    table
        .pairs::<String, Value>()
        .map(|entry| {
            let (k, v) = entry.unwrap();
            (k, scalar(v))
        })
        .collect()
}

#[test]
fn original_full_catalog_constructs_and_initial_state_keeps_ui_fallback_separate() {
    for warm in [false, true] {
        let source = Original::new(warm);
        let (state, fallback) = source.initial(&source.rows);
        let inputs = scalar_map(state.get("input").unwrap());
        let placeholders = scalar_map(state.get("placeholder").unwrap());
        assert_eq!(source.rows.raw_len(), 663);
        assert_eq!(inputs["resistancePenalty"], Scalar::Number(-60.));
        assert_eq!(inputs["enemyIsBoss"], Scalar::Text("Pinnacle".into()));
        assert!(!inputs.contains_key("enemyLevel"));
        assert_eq!(
            scalar(fallback.call(("enemyLevel", "number")).unwrap()),
            Scalar::Number(0.)
        );
        assert_eq!(placeholders["enemyPhysicalDamage"], Scalar::Number(7.));
        assert_eq!(
            scalar(fallback.call(("enemyPhysicalDamage", "number")).unwrap()),
            Scalar::Number(7.)
        );
        assert!(
            source
                .lua
                .globals()
                .get::<Value>("modLib")
                .unwrap()
                .is_nil(),
            "no modifier callback execution is necessary for source initial state"
        );
    }
}

#[test]
fn original_duplicate_assignments_remove_values_and_ui_placeholder_precedence_is_separate() {
    for warm in [false, true] {
        let source = Original::new(warm);
        let rows:Table=source.lua.load(r#"return {
 {var='arbitrary',type='float',defaultState=5.5,defaultPlaceholderState=3.25},
 {var='falseValue',type='check',defaultState=false},
 {var='choice',type='list',defaultState=99,defaultIndex=3,list={{val=0},{val='0'},{val=false},{val='false'}}},
 {var='erased',type='float',defaultState=17,defaultPlaceholderState=2},
 {var='erased',type='float'},
 {var='arbitrary',type='float',defaultPlaceholderState=0,apply=function()error('callbacks must not run')end}
}"#).eval().unwrap();
        let (initial, _) = source.initial(&rows);
        let inputs = scalar_map(initial.get("input").unwrap());
        let placeholders = scalar_map(initial.get("placeholder").unwrap());
        assert!(!inputs.contains_key("erased"));
        assert!(!placeholders.contains_key("erased"));
        assert!(!inputs.contains_key("arbitrary"));
        assert_eq!(placeholders["arbitrary"], Scalar::Number(0.));
        assert_eq!(inputs["choice"], Scalar::Boolean(false));
        assert_eq!(inputs["falseValue"], Scalar::Boolean(false));
        let fallback_placeholders = source.lua.create_table().unwrap();
        fallback_placeholders.set("priority", false).unwrap();
        let ui_defaults = source.lua.create_table().unwrap();
        ui_defaults.set("priority", 99).unwrap();
        ui_defaults.set("uiOnly", "source UI default").unwrap();
        let (_, fallback): (Table, Function) = source
            .defaults
            .call((&rows, source.rounds, fallback_placeholders, ui_defaults))
            .unwrap();
        assert_eq!(
            scalar(fallback.call(("priority", "number")).unwrap()),
            Scalar::Boolean(false)
        );
        assert_eq!(
            scalar(fallback.call(("uiOnly", "boolean")).unwrap()),
            Scalar::Text("source UI default".into())
        );
        for (kind, expected) in [
            ("number", Scalar::Number(0.)),
            ("boolean", Scalar::Boolean(false)),
            ("string", Scalar::Text(String::new())),
        ] {
            assert_eq!(scalar(fallback.call(("missing", kind)).unwrap()), expected);
        }
        assert!(
            fallback
                .call::<Value>(("missing", "unknownType"))
                .unwrap()
                .is_nil()
        );
        assert!(
            !inputs.contains_key("uiOnly"),
            "UI fallback never creates an input assignment"
        );
    }
}

fn exact_scalar(actual: &Scalar, expected: &Scalar) {
    match (actual, expected) {
        (Scalar::Number(a), Scalar::Number(b)) => assert_eq!(a.to_bits(), b.to_bits()),
        _ => assert_eq!(actual, expected),
    }
}
fn exact_optional(actual: Value, expected: &Option<Scalar>) {
    match expected {
        Some(value) => exact_scalar(&scalar(actual), value),
        None => assert!(actual.is_nil()),
    }
}
fn exact_map(actual: Table, expected: &BTreeMap<String, Scalar>) {
    let actual = scalar_map(actual);
    assert_eq!(actual.len(), expected.len());
    for (key, value) in expected {
        exact_scalar(&actual[key], value)
    }
}
fn span_bytes(span: &ConfigSourceSpan) -> String {
    sources()[span.path.as_str()]
        .split_inclusive('\n')
        .skip(span.line as usize - 1)
        .take((span.end_line - span.line + 1) as usize)
        .collect()
}
fn exact_span(span: &ConfigSourceSpan) {
    assert!(span.line > 0 && span.end_line >= span.line);
    let bytes = span_bytes(span);
    assert_eq!(
        format!("{:x}", Sha256::digest(bytes.as_bytes())),
        span.sha256
    );
}
fn exact_metadata(actual: Value, expected: &ConfigMetadataValue) {
    match expected {
        ConfigMetadataValue::Boolean(value) => assert_eq!(actual.as_boolean(), Some(*value)),
        ConfigMetadataValue::Number(value) => {
            exact_scalar(&scalar(actual), &Scalar::Number(*value))
        }
        ConfigMetadataValue::Text(value) => assert_eq!(
            actual.as_string().unwrap().to_str().unwrap().as_ref(),
            value
        ),
        ConfigMetadataValue::Array(values) => {
            let table = actual.as_table().unwrap();
            assert_eq!(table.clone().pairs::<Value, Value>().count(), values.len());
            for (i, value) in values.iter().enumerate() {
                exact_metadata(table.get(i + 1).unwrap(), value)
            }
        }
        ConfigMetadataValue::Object(values) => {
            exact_metadata_map(actual.as_table().unwrap(), values, &[])
        }
        ConfigMetadataValue::Callback(span) => {
            let info = actual.as_function().unwrap().info();
            assert_eq!(
                info.source.as_deref(),
                Some(format!("@{}", span.path).as_str())
            );
            assert_eq!(info.line_defined, Some(span.line as usize));
            assert_eq!(info.last_line_defined, Some(span.end_line as usize));
            exact_span(span);
        }
    }
}
fn exact_metadata_map(
    table: &Table,
    expected: &BTreeMap<String, ConfigMetadataValue>,
    structural: &[&str],
) {
    let keys: BTreeSet<_> = table
        .clone()
        .pairs::<String, Value>()
        .map(|entry| entry.unwrap().0)
        .filter(|key| !structural.contains(&key.as_str()))
        .collect();
    assert_eq!(keys, expected.keys().cloned().collect());
    for (key, value) in expected {
        exact_metadata(table.get(key.as_str()).unwrap(), value)
    }
}
fn widget(name: &str) -> ConfigWidgetKind {
    match name {
        "check" => ConfigWidgetKind::Check,
        "count" => ConfigWidgetKind::Count,
        "countAllowZero" => ConfigWidgetKind::CountAllowZero,
        "integer" => ConfigWidgetKind::Integer,
        "float" => ConfigWidgetKind::Float,
        "list" => ConfigWidgetKind::List,
        "text" => ConfigWidgetKind::Text,
        _ => panic!("unknown original widget {name}"),
    }
}

#[test]
fn every_bundled_definition_option_default_and_callback_matches_independent_original_rows() {
    let snapshot = bundled_snapshot().unwrap();
    let catalog = snapshot.configuration();
    assert_eq!(catalog.data().capability, ConfigCapability::MetadataOnly);
    assert_eq!(
        catalog.data().source.upstream_revision,
        source::UPSTREAM_REVISION
    );
    for (path, sha) in &catalog.data().source.files {
        assert_eq!(sha, &source::expected_file_sha256(path).unwrap());
    }
    for (span, declaration) in [
        (
            &catalog.data().source.create_config_set,
            "function ConfigTabClass:CreateConfigSet(",
        ),
        (
            &catalog.data().source.get_default_state,
            "function ConfigTabClass:GetDefaultState(",
        ),
    ] {
        exact_span(span);
        assert!(span_bytes(span).starts_with(declaration));
        let bytes = span_bytes(span);
        let lines = bytes.lines().collect::<Vec<_>>();
        assert_eq!(lines.last(), Some(&"end"));
        assert_eq!(
            lines.iter().position(|line| *line == "end"),
            Some(lines.len() - 1),
            "span must end at the original function end"
        );
    }
    for warm in [false, true] {
        let source = Original::new(warm);
        assert_eq!(
            source.rows.raw_len(),
            catalog.data().source_table_rows as usize
        );
        let mut observed = Vec::new();
        for (index, row) in source
            .rows
            .clone()
            .sequence_values::<Table>()
            .map(Result::unwrap)
            .enumerate()
        {
            if row.get::<Option<String>>("var").unwrap().is_some() {
                observed.push((index + 1, row));
            }
        }
        assert_eq!(observed.len(), catalog.definitions().len());
        let mut by_key = BTreeMap::<String, Vec<String>>::new();
        for (order, ((index, row), definition)) in
            observed.iter().zip(catalog.definitions()).enumerate()
        {
            assert_eq!(definition.source_table_index, *index as u32);
            assert_eq!(definition.source_variable_order, order as u32 + 1);
            assert_eq!(definition.key, row.get::<String>("var").unwrap());
            assert_eq!(
                definition.widget,
                widget(&row.get::<String>("type").unwrap())
            );
            assert_eq!(
                definition.label,
                row.get::<Option<String>>("label").unwrap()
            );
            exact_optional(row.get("defaultState").unwrap(), &definition.defaults.input);
            exact_optional(
                row.get("defaultPlaceholderState").unwrap(),
                &definition.defaults.placeholder,
            );
            assert_eq!(
                definition.defaults.option_index,
                row.get::<Option<u32>>("defaultIndex").unwrap()
            );
            let options = row.get::<Option<Table>>("list").unwrap();
            assert_eq!(
                definition.options.len(),
                options.as_ref().map_or(0, Table::raw_len)
            );
            for (option_index, option) in definition.options.iter().enumerate() {
                let actual: Table = options.as_ref().unwrap().get(option_index + 1).unwrap();
                assert_eq!(option.index, option_index as u32 + 1);
                exact_scalar(&scalar(actual.get("val").unwrap()), &option.value);
                assert_eq!(option.label, actual.get::<Option<String>>("label").unwrap());
                exact_metadata_map(&actual, &option.metadata, &["val", "label"]);
            }
            exact_metadata_map(
                row,
                &definition.metadata,
                &[
                    "var",
                    "type",
                    "label",
                    "list",
                    "defaultState",
                    "defaultPlaceholderState",
                    "defaultIndex",
                ],
            );
            if let Some(quest) = &definition.source.quest {
                let quests: Table = source
                    .lua
                    .globals()
                    .get::<Table>("data")
                    .unwrap()
                    .get("questRewards")
                    .unwrap();
                let actual: Table = quests.get(quest.source_index).unwrap();
                exact_metadata_map(&actual, &quest.record, &[]);
                let expected_key = format!(
                    "quest{}{}{}",
                    actual.get::<String>("Description").unwrap(),
                    actual.get::<String>("Area").unwrap(),
                    actual.get::<String>("Info").unwrap()
                );
                assert_eq!(definition.key, expected_key);
                assert_eq!(definition.source.location.path, "src/Data/QuestRewards.lua");
                exact_span(definition.source.generator.as_ref().unwrap());
            } else {
                assert_eq!(
                    definition.source.location.path,
                    "src/Modules/ConfigOptions.lua"
                );
                assert!(
                    sources()[definition.source.location.path.as_str()]
                        .lines()
                        .nth(definition.source.location.line as usize - 1)
                        .unwrap()
                        .contains(&definition.key)
                );
                assert!(definition.source.generator.is_none());
            }
            by_key
                .entry(definition.key.clone())
                .or_default()
                .push(definition.id.clone());
            assert_eq!(catalog.definition(&definition.id), Some(definition));
        }
        for (key, ids) in by_key {
            assert_eq!(
                catalog
                    .definitions_for_key(&key)
                    .map(|row| row.id.clone())
                    .collect::<Vec<_>>(),
                ids
            );
        }
        let (initial, _) = source.initial(&source.rows);
        let prepared = catalog.initial_state();
        exact_map(initial.get("input").unwrap(), &prepared.inputs);
        exact_map(initial.get("placeholder").unwrap(), &prepared.placeholders);
        assert!(
            !prepared.inputs.contains_key("enemyLevel"),
            "initial state does not execute enemyIsBoss callback defaults"
        );
    }
}

#[derive(Clone)]
struct AuthoredRow {
    key: String,
    widget: ConfigWidgetKind,
    input: Option<Scalar>,
    placeholder: Option<Scalar>,
    option_index: Option<u32>,
    options: Vec<Scalar>,
}
fn set_scalar(table: &Table, key: impl mlua::IntoLua, value: &Scalar) {
    match value {
        Scalar::Boolean(value) => table.set(key, *value).unwrap(),
        Scalar::Number(value) => table.set(key, *value).unwrap(),
        Scalar::Text(value) => table.set(key, value.as_str()).unwrap(),
    }
}
fn custom_row(key: &str, input: Option<Scalar>, placeholder: Option<Scalar>) -> AuthoredRow {
    AuthoredRow {
        key: key.into(),
        widget: ConfigWidgetKind::Float,
        input,
        placeholder,
        option_index: None,
        options: vec![],
    }
}
fn custom_catalog_and_source(
    original: &Original,
    rows: &[AuthoredRow],
) -> (ConfigDefinitionCatalog, Table) {
    let snapshot = bundled_snapshot().unwrap();
    let mut data = snapshot.configuration().data().clone();
    let template_source = data.definitions[0].source.clone();
    data.source_table_rows = rows.len() as u32;
    data.definitions.clear();
    let source_rows = original.lua.create_table().unwrap();
    for (index, row) in rows.iter().enumerate() {
        let source_row = original.lua.create_table().unwrap();
        source_row.set("var", row.key.as_str()).unwrap();
        source_row
            .set(
                "type",
                match row.widget {
                    ConfigWidgetKind::List => "list",
                    ConfigWidgetKind::Check => "check",
                    _ => "float",
                },
            )
            .unwrap();
        if let Some(value) = &row.input {
            set_scalar(&source_row, "defaultState", value)
        }
        if let Some(value) = &row.placeholder {
            set_scalar(&source_row, "defaultPlaceholderState", value)
        }
        if let Some(value) = row.option_index {
            source_row.set("defaultIndex", value).unwrap()
        }
        if !row.options.is_empty() {
            let options = original.lua.create_table().unwrap();
            for (i, value) in row.options.iter().enumerate() {
                let option = original.lua.create_table().unwrap();
                set_scalar(&option, "val", value);
                options.set(i + 1, option).unwrap()
            }
            source_row.set("list", options).unwrap();
        }
        // The source consumer must never execute callbacks to prepare initial maps.
        source_row
            .set(
                "apply",
                original
                    .lua
                    .create_function(|_, ()| -> mlua::Result<()> {
                        panic!("initial state executed an unmodeled callback")
                    })
                    .unwrap(),
            )
            .unwrap();
        source_rows.set(index + 1, source_row).unwrap();
        data.definitions.push(ConfigDefinition {
            id: format!("caller-defined-occurrence-{index}"),
            key: row.key.clone(),
            source_table_index: index as u32 + 1,
            source_variable_order: index as u32 + 1,
            widget: row.widget,
            scalar_kinds: if row.widget == ConfigWidgetKind::List {
                row.options
                    .iter()
                    .map(ConfigScalarKind::of)
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect()
            } else if row.widget == ConfigWidgetKind::Check {
                vec![ConfigScalarKind::Boolean]
            } else {
                vec![ConfigScalarKind::Number]
            },
            label: Some(format!("Caller-authored row {index}")),
            options: row
                .options
                .iter()
                .enumerate()
                .map(|(i, value)| ConfigOption {
                    index: i as u32 + 1,
                    value: value.clone(),
                    label: Some(format!("Caller option {i}")),
                    metadata: BTreeMap::new(),
                })
                .collect(),
            defaults: ConfigDeclaredDefaults {
                input: row.input.clone(),
                placeholder: row.placeholder.clone(),
                option_index: row.option_index,
            },
            source: template_source.clone(),
            metadata: BTreeMap::new(),
        });
    }
    (ConfigDefinitionCatalog::new(data).unwrap(), source_rows)
}
#[test]
fn injected_catalog_names_order_options_and_nil_defaults_match_original_assignment_semantics() {
    let base = vec![
        custom_row(
            "changed twice",
            Some(Scalar::Number(5.5)),
            Some(Scalar::Number(3.25)),
        ),
        AuthoredRow {
            key: "boolean false".into(),
            widget: ConfigWidgetKind::Check,
            input: Some(Scalar::Boolean(false)),
            placeholder: None,
            option_index: None,
            options: vec![],
        },
        AuthoredRow {
            key: "typed choice".into(),
            widget: ConfigWidgetKind::List,
            input: Some(Scalar::Number(99.)),
            placeholder: Some(Scalar::Text("retained hint".into())),
            option_index: Some(3),
            options: vec![
                Scalar::Number(-0.),
                Scalar::Text("0".into()),
                Scalar::Boolean(false),
                Scalar::Text("false".into()),
                Scalar::Number(0.),
            ],
        },
        custom_row(
            "removed by later nil",
            Some(Scalar::Number(17.)),
            Some(Scalar::Number(2.)),
        ),
        custom_row("removed by later nil", None, None),
        custom_row(
            "subnormal",
            Some(Scalar::Number(f64::from_bits(1))),
            Some(Scalar::Number(-0.)),
        ),
        custom_row("changed twice", None, Some(Scalar::Number(0.))),
        custom_row(
            "widget does not coerce source defaults",
            Some(Scalar::Boolean(false)),
            Some(Scalar::Text("source text fallback".into())),
        ),
    ];
    for warm in [false, true] {
        let original = Original::new(warm);
        for variant in 0..5 {
            let mut rows = base.clone();
            match variant {
                0 => {}
                1 => rows.reverse(),
                2 => {
                    for row in &mut rows {
                        row.key = format!("completely renamed {}", row.key);
                        row.options.reverse();
                    }
                }
                3 => {
                    rows.rotate_left(3);
                    for row in &mut rows {
                        if !row.options.is_empty() {
                            row.option_index = Some(2);
                        }
                    }
                }
                4 => {
                    for row in &mut rows {
                        row.input = None;
                        row.placeholder = None;
                        if !row.options.is_empty() {
                            row.option_index = Some(5)
                        }
                    }
                }
                _ => unreachable!(),
            }
            let (catalog, source_rows) = custom_catalog_and_source(&original, &rows);
            let (actual, _) = original.initial(&source_rows);
            let prepared = catalog.initial_state();
            exact_map(actual.get("input").unwrap(), &prepared.inputs);
            exact_map(actual.get("placeholder").unwrap(), &prepared.placeholders);
            assert_eq!(catalog.data().capability, ConfigCapability::MetadataOnly);
            if variant == 0 {
                assert!(!prepared.inputs.contains_key("removed by later nil"));
                assert!(!prepared.placeholders.contains_key("removed by later nil"));
                assert!(!prepared.inputs.contains_key("changed twice"));
                assert_eq!(prepared.inputs["typed choice"], Scalar::Boolean(false));
                let options = &catalog
                    .definitions_for_key("typed choice")
                    .next()
                    .unwrap()
                    .options;
                for observed in [
                    Scalar::Boolean(false),
                    Scalar::Number(0.),
                    Scalar::Text("0".into()),
                    Scalar::Text("false".into()),
                ] {
                    let target = original.lua.create_table().unwrap();
                    set_scalar(&target, "value", &observed);
                    let equal:Function=original.lua.load("return function(options,target)local result={} for i,option in ipairs(options)do if option.val==target.value then result[#result+1]=i end end return result end").eval().unwrap();
                    let matched: Table = equal
                        .call((
                            source_rows
                                .get::<Table>(3)
                                .unwrap()
                                .get::<Table>("list")
                                .unwrap(),
                            target,
                        ))
                        .unwrap();
                    assert_eq!(
                        options
                            .iter()
                            .filter(|option| option.matches(&observed))
                            .map(|option| option.index)
                            .collect::<Vec<_>>(),
                        matched
                            .sequence_values::<u32>()
                            .map(Result::unwrap)
                            .collect::<Vec<_>>()
                    );
                }
            }
        }
    }
}
