//! Declared finite caller contexts exercise the unchanged complete original method.
use super::*;
fn table(fields: impl IntoIterator<Item = (&'static str, Field)>) -> Metadata {
    Metadata {
        fields: fields.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        indexed: BTreeMap::new(),
    }
}
fn text(v: &str) -> Field {
    Field::Text(v.into())
}
fn item(kind: &str) -> Metadata {
    table([
        ("type", text(kind)),
        ("rarity", text("NORMAL")),
        ("baseName", text(kind)),
        (
            "base",
            Field::Table(table([
                ("type", text(kind)),
                ("tags", Field::Table(Metadata::default())),
            ])),
        ),
    ])
}
fn base_mut(item: &mut Metadata) -> &mut Metadata {
    let Field::Table(t) = item.fields.get_mut("base").unwrap() else {
        panic!()
    };
    t
}
fn root() -> Metadata {
    let set = table([
        (
            "Weapon 1",
            Field::Table(table([("selItemId", Field::Number(1.0))])),
        ),
        (
            "Weapon 1 Swap",
            Field::Table(table([("selItemId", Field::Number(1.0))])),
        ),
    ]);
    table([
        ("activeItemSet", Field::Table(set)),
        ("items", Field::Table(Metadata::default())),
        ("treeNodes", Field::Table(Metadata::default())),
        ("specNodes", Field::Table(Metadata::default())),
    ])
}
fn member_mut<'a>(root: &'a mut Metadata, key: &str) -> &'a mut Metadata {
    let Field::Table(t) = root.fields.get_mut(key).unwrap() else {
        panic!()
    };
    t
}
fn flags(a: Field, b: Field, c: Field) -> Field {
    Field::Table(table([
        ("giantsBlood", a),
        ("instrumentsOfPower", b),
        ("lordOfTheWilds", c),
    ]))
}
#[derive(Clone)]
struct Case {
    label: String,
    item: Option<Field>,
    slot: String,
    root: Metadata,
    explicit_set: Option<Field>,
    flags: Option<Field>,
    alias: bool,
    calcs: bool,
    query_results: Metadata,
    query_failure: Option<String>,
}
impl Case {
    fn new(label: &str, kind: &str, slot: &str) -> Self {
        Self {
            label: label.into(),
            item: Some(Field::Table(item(kind))),
            slot: slot.into(),
            root: root(),
            explicit_set: None,
            flags: None,
            alias: false,
            calcs: false,
            query_results: Metadata::default(),
            query_failure: None,
        }
    }
    fn item_mut(&mut self) -> &mut Metadata {
        let Some(Field::Table(t)) = &mut self.item else {
            panic!()
        };
        t
    }
}
fn evaluate(lua: &Lua, program: &SlotValidityProgram, method: &Function, case: Case) -> Json {
    let source_root = lua_table(lua, &case.root);
    let receiver = lua.create_table().unwrap();
    receiver
        .raw_set("items", source_root.raw_get::<LuaValue>("items").unwrap())
        .unwrap();
    receiver
        .raw_set(
            "activeItemSet",
            source_root.raw_get::<LuaValue>("activeItemSet").unwrap(),
        )
        .unwrap();
    let tree = lua.create_table().unwrap();
    tree.raw_set(
        "nodes",
        source_root.raw_get::<LuaValue>("treeNodes").unwrap(),
    )
    .unwrap();
    let spec = lua.create_table().unwrap();
    spec.raw_set("tree", tree).unwrap();
    spec.raw_set(
        "nodes",
        source_root.raw_get::<LuaValue>("specNodes").unwrap(),
    )
    .unwrap();
    let build = lua.create_table().unwrap();
    build.raw_set("spec", spec).unwrap();
    receiver.raw_set("build", build).unwrap();
    let source_item = case
        .item
        .as_ref()
        .map_or(LuaValue::Nil, |v| lua_value(lua, v));
    let source_set = case
        .explicit_set
        .as_ref()
        .map_or(LuaValue::Nil, |v| lua_value(lua, v));
    let source_flags = case
        .flags
        .as_ref()
        .map_or(LuaValue::Nil, |v| lua_value(lua, v));
    // The substituted dependency is separate from the finite structural graph.
    let projected_receiver = lua.create_table().unwrap();
    projected_receiver
        .raw_set("items", source_root.raw_get::<LuaValue>("items").unwrap())
        .unwrap();
    projected_receiver
        .raw_set(
            "activeItemSet",
            source_root.raw_get::<LuaValue>("activeItemSet").unwrap(),
        )
        .unwrap();
    projected_receiver
        .raw_set(
            "spec",
            receiver
                .raw_get::<Table>("build")
                .unwrap()
                .raw_get::<Table>("spec")
                .unwrap(),
        )
        .unwrap();
    let calls = Rc::new(RefCell::new(Vec::<String>::new()));
    let mut dependency = None;
    if case.calcs {
        let db = lua.create_table().unwrap();
        let expected = db.clone();
        let trace = calls.clone();
        let answers = case.query_results.clone();
        let failure = case.query_failure.clone();
        let flag = lua
            .create_function(
                move |lua, (receiver, cfg, name): (Table, LuaValue, String)| {
                    assert_eq!(receiver, expected);
                    assert!(matches!(cfg, LuaValue::Nil));
                    trace.borrow_mut().push(name.clone());
                    if failure.as_deref() == Some(name.as_str()) {
                        return Err(mlua::Error::RuntimeError(
                            "directed original Flag failure".into(),
                        ));
                    }
                    Ok(answers
                        .fields
                        .get(&name)
                        .map_or(LuaValue::Nil, |v| lua_value(lua, v)))
                },
            )
            .unwrap();
        db.raw_set("Flag", flag.clone()).unwrap();
        let env = lua.create_table().unwrap();
        env.raw_set("modDB", db.clone()).unwrap();
        let calcs = lua.create_table().unwrap();
        calcs.raw_set("mainEnv", env).unwrap();
        receiver
            .raw_get::<Table>("build")
            .unwrap()
            .raw_set("calcsTab", calcs)
            .unwrap();
        dependency = Some((db, flag));
    }
    let roots = [
        LuaValue::Table(projected_receiver),
        source_item.clone(),
        source_set.clone(),
        source_flags.clone(),
    ];
    let before = source_graph(&roots);
    let mut ctx = Context {
        root: &case.root,
        flags: &case.query_results,
        calcs: case.calcs,
        calls: vec![],
        failure: case.query_failure.as_deref(),
    };
    let native_item = case
        .item
        .as_ref()
        .map_or(Ok(Value::Nil), Value::metadata)
        .unwrap();
    let request = SlotValidityRequest {
        item: native_item,
        slot_name: &case.slot,
        item_set: case
            .explicit_set
            .as_ref()
            .map_or(Ok(Value::Nil), Value::metadata)
            .unwrap(),
        flag_state: case
            .flags
            .as_ref()
            .map_or(Ok(Value::Nil), Value::metadata)
            .unwrap(),
    };
    let outcome = compare(
        program,
        method,
        receiver.clone(),
        source_item.clone(),
        &case.slot,
        source_set,
        source_flags,
        request,
        &mut ctx,
        &case.label,
    );
    assert_eq!(
        before,
        source_graph(&roots),
        "{} source caller state mutated",
        case.label
    );
    assert_eq!(
        *calls.borrow(),
        ctx.calls,
        "{} exact substituted dependency order/error prefix",
        case.label
    );
    if let Some((db, flag)) = &dependency {
        assert_eq!(db.raw_get::<Function>("Flag").unwrap(), *flag);
    }
    let dependency_trace = calls.borrow().clone();
    if case.alias {
        let raw: MultiValue = method
            .call((receiver, source_item.clone(), case.slot.as_str()))
            .unwrap();
        let LuaValue::Table(returned) = raw.front().unwrap() else {
            panic!()
        };
        let LuaValue::Table(source_item) = source_item else {
            panic!()
        };
        let original: Table = source_item
            .raw_get::<Table>("base")
            .unwrap()
            .raw_get::<Table>("tags")
            .unwrap()
            .raw_get("onehand")
            .unwrap();
        assert_eq!(returned, &original);
        let result = program
            .check(
                SlotValidityRequest {
                    item: native_item,
                    slot_name: &case.slot,
                    item_set: Value::Nil,
                    flag_state: Value::Nil,
                },
                &mut ctx,
            )
            .unwrap();
        let SlotValidityResult::Value(result) = result else {
            panic!()
        };
        assert!(
            result.same_identity(
                native_item
                    .field("base")
                    .unwrap()
                    .field("tags")
                    .unwrap()
                    .field("onehand")
                    .unwrap()
            )
        );
    }
    json!({"label":case.label,"slot":case.slot,"outcome":outcome,"same_declared_finite_inputs":true,"input_graph_sha256":hash(before.to_string().as_bytes()),"read_only_context_unchanged":true,"raw_return_alias_checked":case.alias,"source_fed_context":true,"native_preparation":false,"dependency_substituted":case.calcs,"source_dependency_queries":dependency_trace,"query_failure":case.query_failure,"original_actor_flag_semantics_claimed":false})
}
pub(super) fn run(lua: &Lua, program: &SlotValidityProgram, method: &Function) -> Json {
    let mut cases = Vec::new();
    for slot in [
        "Ring 1",
        "Ring 2",
        "Ring 3",
        "Ring 1 Swap",
        "Unrecognized",
        "Weapon",
        "Weapon 1",
        "Weapon 1 Swap",
    ] {
        cases.push(Case::new(&format!("generic slot {slot}"), "Ring", slot));
    }
    for (kind, slot, subtype) in [
        ("Any", "Arm 1", "Transcendent Arm"),
        ("Any", "Leg 2", "Transcendent Leg"),
        ("Any", "Arm 1", "Transcendent Leg"),
    ] {
        let mut c = Case::new("transcendent subtype route", kind, slot);
        base_mut(c.item_mut())
            .fields
            .insert("subType".into(), text(subtype));
        cases.push(c);
    }
    for name in ["Life Flask", "Mana Flask", "Unknown Flask"] {
        for slot in [
            "Flask 1",
            "Flask 2",
            "Flask 10",
            "Flask 1 Swap",
            "Unrecognized",
        ] {
            let mut c = Case::new(&format!("flask {name} {slot}"), "Flask", slot);
            c.item_mut().fields.insert("baseName".into(), text(name));
            cases.push(c);
        }
    }
    for value in [
        Field::Boolean(false),
        Field::Number(0.0),
        Field::Number(-0.0),
        text(""),
        text("tag"),
        Field::Table(table([("payload", Field::Number(2.5))])),
    ] {
        let mut c = Case::new("raw primary tag result", "Mace", "Weapon 1");
        c.alias = matches!(value, Field::Table(_));
        base_mut(c.item_mut())
            .fields
            .insert("tags".into(), Field::Table(table([("onehand", value)])));
        cases.push(c);
    }
    let mut c = Case::new("primary last false", "Mace", "Weapon");
    base_mut(c.item_mut()).fields.insert(
        "tags".into(),
        Field::Table(table([
            ("onehand", Field::Boolean(false)),
            ("twohand", Field::Boolean(false)),
        ])),
    );
    cases.push(c);
    for rarity in ["NORMAL", "UNIQUE", "RELIC"] {
        for node in [
            Metadata::default(),
            table([("sinister", Field::Boolean(true))]),
            table([("containJewelSocket", Field::Boolean(true))]),
            table([("charmSocket", Field::Boolean(true))]),
            table([(
                "expansionJewel",
                Field::Table(table([("size", Field::Number(2.0))])),
            )]),
            table([(
                "expansionJewel",
                Field::Table(table([("size", Field::Number(1.0))])),
            )]),
        ] {
            let mut c = Case::new("jewel node guards", "Jewel", "Jewel 42");
            c.item_mut().fields.insert("rarity".into(), text(rarity));
            member_mut(&mut c.root, "treeNodes")
                .indexed
                .insert(42, Field::Table(node));
            cases.push(c);
        }
    }
    cases.push(Case::new("missing jewel node", "Jewel", "Jewel 42"));
    cases.push(Case::new(
        "missing node before malformed item",
        "Unknown",
        "Jewel 42",
    ));
    let mut c = Case::new("effective node fallback", "Jewel", "Jewel 42");
    member_mut(&mut c.root, "treeNodes")
        .indexed
        .insert(42, Field::Boolean(false));
    member_mut(&mut c.root, "specNodes")
        .indexed
        .insert(42, Field::Table(Metadata::default()));
    cases.push(c);
    let mut c = Case::new("tree node precedes effective node", "Jewel", "Jewel 42");
    member_mut(&mut c.root, "treeNodes").indexed.insert(
        42,
        Field::Table(table([("sinister", Field::Boolean(true))])),
    );
    member_mut(&mut c.root, "specNodes")
        .indexed
        .insert(42, Field::Table(Metadata::default()));
    c.item_mut().fields.insert("rarity".into(), text("UNIQUE"));
    cases.push(c);
    for size in [
        Field::Number(0.0),
        Field::Number(1.0),
        Field::Number(2.0),
        text("1"),
        Field::Boolean(false),
    ] {
        let mut c = Case::new("cluster size comparison", "Jewel", "Jewel 42");
        member_mut(&mut c.root, "treeNodes").indexed.insert(
            42,
            Field::Table(table([(
                "expansionJewel",
                Field::Table(table([("size", Field::Number(1.0))])),
            )])),
        );
        c.item_mut().fields.insert(
            "clusterJewel".into(),
            Field::Table(table([("sizeIndex", size)])),
        );
        cases.push(c);
    }
    for subtype in [None, Some("Charm"), Some("Cluster")] {
        let mut c = Case::new("contained jewel subtype", "Jewel", "Jewel 42");
        member_mut(&mut c.root, "treeNodes").indexed.insert(
            42,
            Field::Table(table([("containJewelSocket", Field::Boolean(true))])),
        );
        if let Some(s) = subtype {
            base_mut(c.item_mut())
                .fields
                .insert("subType".into(), text(s));
        }
        cases.push(c);
    }
    for rarity in ["NORMAL", "UNIQUE", "RELIC"] {
        for restriction in [
            None,
            Some(Field::Boolean(false)),
            Some(Field::Table(table([("Ruby", Field::Boolean(true))]))),
            Some(Field::Table(table([("Ruby", Field::Boolean(false))]))),
        ] {
            let mut c = Case::new("parent jewel restriction", "Jewel", "Ring 1 Jewel Socket 1");
            c.item_mut().fields.insert("baseName".into(), text("Ruby"));
            c.item_mut().fields.insert("rarity".into(), text(rarity));
            let mut parent = item("Ring");
            if let Some(v) = restriction {
                parent.fields.insert("canSocketJewelBase".into(), v);
            }
            member_mut(&mut c.root, "items")
                .indexed
                .insert(7, Field::Table(parent));
            member_mut(&mut c.root, "activeItemSet").fields.insert(
                "Ring 1".into(),
                Field::Table(table([("selItemId", Field::Number(7.0))])),
            );
            cases.push(c);
        }
    }
    for primary in [
        "Bow", "Talisman", "Staff", "Wand", "Sceptre", "Sword", "Unarmed",
    ] {
        for offhand in ["Quiver", "Sceptre", "Focus", "Shield", "Spear", "Mace"] {
            for mask in [0u8, 7] {
                let mut c = Case::new(
                    &format!("offhand {primary} {offhand} flags {mask}"),
                    offhand,
                    "Weapon 2",
                );
                let mut p = item(primary);
                let tags = table([
                    (
                        "onehand",
                        Field::Boolean(
                            primary == "Sword" || primary == "Wand" || primary == "Sceptre",
                        ),
                    ),
                    ("mace", Field::Boolean(primary == "Mace")),
                    ("sword", Field::Boolean(primary == "Sword")),
                ]);
                base_mut(&mut p)
                    .fields
                    .insert("tags".into(), Field::Table(tags));
                if primary != "Unarmed" {
                    member_mut(&mut c.root, "items")
                        .indexed
                        .insert(1, Field::Table(p));
                }
                base_mut(c.item_mut()).fields.insert(
                    "tags".into(),
                    Field::Table(table([
                        ("one_hand_weapon", Field::Boolean(true)),
                        ("mace", Field::Boolean(offhand == "Mace")),
                    ])),
                );
                c.flags = Some(flags(
                    Field::Boolean(mask & 1 != 0),
                    Field::Boolean(mask & 2 != 0),
                    Field::Boolean(mask & 4 != 0),
                ));
                cases.push(c);
            }
        }
    }
    for slot in ["Weapon 2", "Weapon 2 Swap"] {
        let mut c = Case::new("early default flags with no calcs", "Mace", slot);
        let mut p = item("Mace");
        base_mut(&mut p).fields.insert(
            "tags".into(),
            Field::Table(table([("mace", Field::Boolean(true))])),
        );
        member_mut(&mut c.root, "items")
            .indexed
            .insert(1, Field::Table(p));
        base_mut(c.item_mut()).fields.insert(
            "tags".into(),
            Field::Table(table([("mace", Field::Number(0.0))])),
        );
        cases.push(c);
    }
    let mut c = Case::new("explicit false selects active set", "Shield", "Weapon 2");
    c.explicit_set = Some(Field::Boolean(false));
    cases.push(c);
    let mut c = Case::new("explicit set takes precedence", "Shield", "Weapon 2");
    c.explicit_set = Some(Field::Table(Metadata::default()));
    cases.push(c);
    for (label, slot) in [
        ("nil Item ordinary source error", "Ring 1"),
        ("nil Item missing-node short circuit", "Jewel 42"),
    ] {
        let mut c = Case::new(label, "Ring", slot);
        c.item = None;
        cases.push(c);
    }
    let mut c = Case::new(
        "malformed flag state reached before bow answer",
        "Quiver",
        "Weapon 2",
    );
    c.flags = Some(Field::Boolean(true));
    member_mut(&mut c.root, "items")
        .indexed
        .insert(1, Field::Table(item("Bow")));
    cases.push(c);
    let mut c = Case::new("malformed base tags source error", "Mace", "Weapon");
    base_mut(c.item_mut())
        .fields
        .insert("tags".into(), Field::Boolean(false));
    cases.push(c);
    let mut c = Case::new("nil base source error", "Ring", "Ring 1");
    c.item_mut().fields.remove("base");
    cases.push(c);
    for failing in [None, Some("InstrumentsOfPower"), Some("LordOfTheWilds")] {
        let mut c = Case::new(
            "ordered substituted actor queries before Bow answer",
            "Quiver",
            "Weapon 2",
        );
        c.calcs = true;
        c.query_failure = failing.map(str::to_owned);
        c.query_results = table([
            ("GiantsBlood", Field::Boolean(false)),
            ("InstrumentsOfPower", Field::Boolean(false)),
            ("LordOfTheWilds", Field::Boolean(false)),
        ]);
        member_mut(&mut c.root, "items")
            .indexed
            .insert(1, Field::Table(item("Bow")));
        cases.push(c);
    }
    let mut c = Case::new(
        "explicit flag state bypasses substituted query failure",
        "Quiver",
        "Weapon 2",
    );
    c.calcs = true;
    c.query_failure = Some("GiantsBlood".into());
    c.flags = Some(flags(
        Field::Boolean(false),
        Field::Boolean(false),
        Field::Boolean(false),
    ));
    member_mut(&mut c.root, "items")
        .indexed
        .insert(1, Field::Table(item("Bow")));
    cases.push(c);
    let rows = cases
        .into_iter()
        .map(|c| evaluate(lua, program, method, c))
        .collect::<Vec<_>>();
    json!({"cases":rows,"scope":{"complete_original_method":true,"derived_finite_precall_inputs":true,"original_saved_build_mutated":false,"source_poststate_used_as_dependency":false,"substituted_flag_dependency_cases_labelled":true,"whole_load_parity":false}})
}
