//! Test-only projection of actual source objects, with production class capture.
use super::*;
const PROBES: &str = include_str!("source_configuration_dispatch.lua");
const SOURCE_PATHS: &[&str] = &[
    "src/Modules/Common.lua",
    "src/Data/Global.lua",
    "src/Modules/ModTools.lua",
    "src/Modules/ConfigOptions.lua",
    "src/Classes/ConfigTab.lua",
    "src/Classes/EditControl.lua",
    "src/Classes/ControlHost.lua",
    "src/Classes/Control.lua",
    "src/Classes/UndoHandler.lua",
    "src/Classes/TooltipHost.lua",
    "src/Classes/ModStore.lua",
    "src/Classes/ModList.lua",
];
fn selection(table: Table, names: &[&str], behavior: bool) -> SourceTableSelection {
    SourceTableSelection {
        table,
        fields: names.iter().map(|s| (*s).into()).collect(),
        indexed: BTreeSet::new(),
        allow_index_fallback: behavior,
        allow_call_fallback: behavior,
    }
}
/// Temporarily expose only the retained originals of our known instrumentation.
/// Class identity and original Common constructor wrappers remain intact. The
/// guard restores instrumentation before source execution, also on capture failure.
struct Instrumentation(Vec<(Table, String, Function)>);
impl Instrumentation {
    fn suspend(registry: &Table, primitives: &Primitives) -> Self {
        let mut saved = Vec::new();
        for (name, methods) in [
            ("ConfigTab", &["ConfigTab", "UpdateLevel"][..]),
            ("EditControl", &["SetPlaceholder"][..]),
        ] {
            let class: Table = registry.raw_get(name).unwrap();
            for method in methods {
                let wrapped: Function = class.raw_get(*method).unwrap();
                assert_eq!(
                    wrapped.info().source.as_deref(),
                    Some("@configuration-source-observation.lua")
                );
                let original = primitives.unwrap(&wrapped, "original");
                class.raw_set(*method, original).unwrap();
                saved.push((class.clone(), (*method).into(), wrapped));
            }
        }
        Self(saved)
    }
}
impl Drop for Instrumentation {
    fn drop(&mut self) {
        for (class, name, function) in self.0.drain(..) {
            class.raw_set(name, function).unwrap();
        }
    }
}
pub struct Captured {
    pub observed: ObservedSourceSession,
    pub compiled: CompiledSourcePrograms,
    pub probe: Function,
    pub names: Table,
    pub unsupported: Json,
    pub functions: BTreeMap<usize, Function>,
    pub round_id: poe_optimizer_data::source_program::SourceCallbackId,
}
pub fn capture(
    lua: &Lua,
    primitives: &Primitives,
    player: &Table,
    enemy: &Table,
    build: &Table,
) -> Captured {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
    let mut texts = BTreeMap::new();
    for path in SOURCE_PATHS {
        texts.insert(
            (*path).into(),
            poe_optimizer_pob::source::read_verified_text(&root, path).unwrap(),
        );
    }
    let probes: Table = lua
        .load(PROBES)
        .set_name("@tests/support/source_configuration_dispatch.lua")
        .eval()
        .unwrap();
    texts.insert(
        "tests/support/source_configuration_dispatch.lua".into(),
        PROBES.into(),
    );
    let probe: Function = probes.raw_get("state").unwrap();
    let globals = lua.globals();
    let registry: Table = globals
        .raw_get::<Table>("common")
        .unwrap()
        .raw_get("classes")
        .unwrap();
    let options: Table = globals
        .raw_get::<Table>("package")
        .unwrap()
        .raw_get::<Table>("loaded")
        .unwrap()
        .raw_get("Modules.ConfigOptions")
        .unwrap();
    let data: Table = globals.raw_get("data").unwrap();
    let mod_lib: Table = globals.raw_get("modLib").unwrap();
    let config: Table = build.raw_get("configTab").unwrap();
    let controls: Table = config.raw_get("varControls").unwrap();
    let mut callbacks = BTreeMap::new();
    let mut functions = BTreeMap::new();
    let mut definitions = vec![
        selection(
            globals.clone(),
            &[
                "_G",
                "data",
                "SkillType",
                "modLib",
                "tostring",
                "tonumber",
                "round",
            ],
            false,
        ),
        selection(mod_lib.clone(), &["createMod"], false),
        selection(
            data.clone(),
            &[
                "misc",
                "monsterConstants",
                "monsterDamageTable",
                "monsterArmourTable",
                "monsterEvasionTable",
                "bossStats",
            ],
            false,
        ),
    ];
    let mut option_selection = selection(options.clone(), &[], false);
    let mut control_selection = selection(controls.clone(), &[], false);
    let mut projections = vec![
        selection(
            build.clone(),
            &["configTab", "characterLevel", "buildFlag"],
            true,
        ),
        selection(
            config.clone(),
            &[
                "build",
                "varControls",
                "configSets",
                "activeConfigSetId",
                "input",
                "placeholder",
                "enemyLevel",
            ],
            true,
        ),
    ];
    let mut instances = vec![player.clone(), enemy.clone(), config.clone()];
    let names = lua.create_table().unwrap();
    for (i, row) in options.sequence_values::<Table>().enumerate() {
        let row = row.unwrap();
        option_selection.indexed.insert((i + 1) as i64);
        definitions.push(selection(row.clone(), &["var"], false));
        if let Value::Function(apply) = row.raw_get::<Value>("apply").unwrap() {
            assert_eq!(
                apply.info().source.as_deref(),
                Some("@configuration-source-observation.lua")
            );
            let original = primitives.unwrap(&apply, "original");
            functions.insert(i + 1, original.clone());
            callbacks.insert(format!("apply.{}", i + 1), original);
        }
        if let Ok(kind) = row.raw_get::<String>("type")
            && ["count", "integer", "countAllowZero", "float"].contains(&kind.as_str())
        {
            let name: String = row.raw_get("var").unwrap();
            if let Value::Table(control) = controls.raw_get::<Value>(name.as_str()).unwrap() {
                control_selection.fields.insert(name.clone());
                names.raw_push(name).unwrap();
                projections.push(selection(
                    control.clone(),
                    &["placeholder", "changeFunc"],
                    true,
                ));
                instances.push(control);
            }
        }
    }
    assert_eq!(callbacks.len(), 537);
    definitions.push(option_selection);
    projections.push(control_selection);
    let sets: Table = config.raw_get("configSets").unwrap();
    for entry in sets.pairs::<Value, Table>() {
        projections.push(selection(
            entry.unwrap().1,
            &["input", "placeholder"],
            false,
        ));
    }
    for list in [player, enemy] {
        let mut selected = selection(list.clone(), &["parent"], true);
        for entry in list.pairs::<Value, Value>() {
            if let Value::Integer(index) = entry.unwrap().0 {
                selected.indexed.insert(index);
            }
        }
        projections.push(selected);
    }
    let mut classes = Vec::new();
    for (name, methods) in [
        ("ModStore", &["NewMod", "ReplaceMod"][..]),
        (
            "ModList",
            &["NewMod", "ReplaceMod", "AddMod", "ReplaceModInternal"][..],
        ),
        ("EditControl", &["SetPlaceholder"][..]),
        ("ConfigTab", &["UpdateLevel"][..]),
        ("ControlHost", &[][..]),
        ("Control", &[][..]),
        ("UndoHandler", &[][..]),
        ("TooltipHost", &[][..]),
    ] {
        classes.push(SourceClassSelection {
            table: registry.raw_get(name).unwrap(),
            methods: methods.iter().map(|s| (*s).into()).collect(),
        });
    }
    let (source, source_names) = classes::inventory(lua, &root, &texts);
    let instrumentation = Instrumentation::suspend(&registry, primitives);
    let observed = primitives
        .observer
        .observe_session_with_classes(
            lua,
            &texts,
            source,
            SourceSessionCaptureRequest {
                callbacks,
                state_roots: BTreeMap::from([
                    ("player".into(), Value::Table(player.clone())),
                    ("enemy".into(), Value::Table(enemy.clone())),
                    ("build".into(), Value::Table(build.clone())),
                    ("names".into(), Value::Table(names.clone())),
                ]),
                definition_roots: BTreeMap::from([
                    ("options".into(), options),
                    ("data".into(), data),
                    ("modLib".into(), mod_lib),
                    ("SkillType".into(), globals.raw_get("SkillType").unwrap()),
                ]),
                definitions: SourceCaptureContext {
                    projections: definitions,
                    environment: Some(SourceEnvironmentSelection {
                        table: globals.clone(),
                        root_name: "original.environment".into(),
                    }),
                    source_names: source_names.clone(),
                },
                state_projections: projections,
            },
            SourceClassCaptureRequest {
                classes,
                callbacks: BTreeMap::from([
                    ("probe.state".into(), probe.clone()),
                    ("original.round".into(), globals.raw_get("round").unwrap()),
                ]),
                definition_roots: BTreeMap::new(),
                allocation: primitives.unwrap(&globals.raw_get("new").unwrap(), "originalNew"),
                source_names,
            },
            instances,
        )
        .unwrap();
    drop(instrumentation);
    let poe_optimizer_data::source_program::SourceSessionValue::Callback(round_id) =
        observed.input().state.values[observed.root_index("original.round").unwrap()]
    else {
        panic!("original round must retain shared callback identity")
    };
    let lowered = lower_from_sources(&texts, observed.owner()).unwrap();
    let unsupported = json!(lowered.unsupported());
    let compiled = CompiledSourcePrograms::new(lowered.catalog()).unwrap();
    Captured {
        observed,
        compiled,
        probe,
        names,
        unsupported,
        functions,
        round_id,
    }
}
