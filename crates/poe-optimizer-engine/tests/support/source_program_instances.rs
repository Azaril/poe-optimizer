//! Coherent observed instance imports reuse class dispatch without allocation.
use super::*;
fn expanded() -> (SourceProgramOwner, CompiledSourcePrograms) {
    let (owner, library) = fixture();
    let mut defs = owner.definitions().unwrap().clone();
    defs.callbacks[19].upvalues = vec![ParserUpvalue {
        name: "instance".into(),
        value: SourceValue::LiveCapture {},
    }];
    let owner = SourceProgramOwner::new_with_closures(
        defs,
        owner.classes().cloned(),
        None,
        SourceClosurePrototypes {
            schema_version: SOURCE_CLOSURE_PROTOTYPES_SCHEMA_VERSION,
            prototypes: vec![SourceClosurePrototype {
                callback: ParserCallbackId(20),
            }],
        },
    )
    .unwrap();
    let mut data = library.catalog().data().clone();
    let programs = [
        (
            18,
            vec![],
            vec![ret(vec![e(ParserProgramExprKind::Get {
                table: Box::new(l(0)),
                key: Box::new(l(1)),
            })])],
        ),
        (
            19,
            vec![ParserProgramBinding::DynamicCall {}],
            vec![ret(vec![e(ParserProgramExprKind::Call {
                call: Box::new(call(
                    0,
                    Some(l(0)),
                    vec![e(ParserProgramExprKind::Call {
                        call: Box::new(call(0, Some(l(1)), vec![l(0), l(2)])),
                    })],
                )),
            })])],
        ),
        (
            20,
            vec![ParserProgramBinding::DynamicMethod {
                key: "Record".into(),
            }],
            vec![ret(vec![e(ParserProgramExprKind::Call {
                call: Box::new(call(
                    0,
                    Some(e(ParserProgramExprKind::Capture { upvalue: 0 })),
                    vec![l(0)],
                )),
            })])],
        ),
        (21, vec![], vec![ret(vec![eq(l(0), l(1))])]),
    ];
    for (id, bindings, body) in programs {
        let mut p = data.programs[0].clone();
        p.callback = ParserCallbackId(id);
        p.parameter_count = 3;
        p.bindings = bindings;
        p.body = body;
        data.programs.push(p);
        data.callbacks.insert(
            ParserCallbackId(id),
            ParserProgramId(data.programs.len() as u32),
        );
    }
    let library =
        CompiledSourcePrograms::new(&SourceProgramCatalog::new(data, owner.clone()).unwrap())
            .unwrap();
    (owner, library)
}
fn bytes(value: &str) -> ProgramValue {
    ProgramValue::Bytes(value.as_bytes().to_vec())
}
fn input(owner: &SourceProgramOwner) -> SourceSessionInput {
    SourceSessionInput {
        traversal: None,
        owner: owner.clone(),
        state: ProgramValueGraph {
            values: vec![
                ProgramValue::Table(ProgramTableId(1)),
                ProgramValue::Table(ProgramTableId(1)),
                ProgramValue::Callback(ParserCallbackId(4)),
                ProgramValue::Closure(SourceSessionClosureId(1)),
            ],
            tables: vec![ProgramTable {
                entries: vec![
                    (bytes("last"), ProgramValue::Number(5.0)),
                    (
                        bytes("savedMethod"),
                        ProgramValue::Callback(ParserCallbackId(4)),
                    ),
                    (
                        ProgramValue::Callback(ParserCallbackId(4)),
                        bytes("method key"),
                    ),
                ],
            }],
        },
        coverage: BTreeMap::from([(
            ProgramTableId(1),
            SourceTableCoverage {
                inventory: SourceTableInventory::Complete,
                known_absent: BTreeSet::new(),
                unavailable: BTreeSet::new(),
                index_fallback: SourceTableIndexFallback::ClassResolved,
                call_fallback: SourceTableCallFallback::NonCallable,
            },
        )]),
        class_bindings: BTreeMap::from([(
            ProgramTableId(1),
            owner.bind_class(SourceClassId(2)).unwrap(),
        )]),
        cells: vec![ProgramValue::Table(ProgramTableId(1))],
        closures: vec![SourceSessionClosure {
            prototype: owner
                .bind_closure_prototype(SourceClosurePrototypeId(1))
                .unwrap(),
            captures: vec![SourceSessionCellId(1)],
        }],
    }
}
fn read(session: &mut ProgramSession, object: &SessionValue, key: ProgramValue) -> SessionValue {
    let key = args(session, vec![key]).remove(0);
    session
        .invoke(ParserCallbackId(18), &[object.clone(), key])
        .unwrap()
        .remove(0)
}
fn scalar(session: &mut ProgramSession, value: &SessionValue) -> ProgramValue {
    session
        .snapshot(std::slice::from_ref(value))
        .unwrap()
        .graph()
        .values[0]
        .clone()
}
fn write(session: &mut ProgramSession, object: &SessionValue, key: &str, value: ProgramValue) {
    let values = args(session, vec![bytes(key), value]);
    session
        .invoke(
            ParserCallbackId(16),
            &[object.clone(), values[0].clone(), values[1].clone()],
        )
        .unwrap();
}
fn rebuild(owner: SourceProgramOwner, library: &CompiledSourcePrograms) -> CompiledSourcePrograms {
    CompiledSourcePrograms::new(
        &SourceProgramCatalog::new(library.catalog().data().clone(), owner).unwrap(),
    )
    .unwrap()
}
#[test]
fn imported_instances_keep_live_cells_method_identity_and_raw_state_without_constructor_synthesis()
{
    let (owner, library) = expanded();
    let (mut session, roots) = library
        .session_from_input(&input(&owner), ProgramLimits::default())
        .unwrap();
    assert_eq!(
        session.allocations().tables,
        1,
        "no allocation/proxy construction"
    );
    for key in ["Object", "Base", "_parentInit"] {
        let value = read(&mut session, &roots[0], bytes(key));
        assert_eq!(scalar(&mut session, &value), ProgramValue::Nil);
    }
    let method = read(&mut session, &roots[0], bytes("Record"));
    let equality = session
        .invoke(ParserCallbackId(21), &[method, roots[2].clone()])
        .unwrap();
    assert_eq!(
        scalar(&mut session, &equality[0]),
        ProgramValue::Boolean(true)
    );
    let key_value = read(
        &mut session,
        &roots[0],
        ProgramValue::Callback(ParserCallbackId(4)),
    );
    assert_eq!(scalar(&mut session, &key_value), bytes("method key"));
    let value = args(&mut session, vec![ProgramValue::Number(42.0)]);
    let returned = session.invoke_callable(&roots[3], &value).unwrap();
    assert_eq!(scalar(&mut session, &returned[0]), bytes("base"));
    let value = read(&mut session, &roots[1], bytes("last"));
    assert_eq!(scalar(&mut session, &value), ProgramValue::Number(42.0));
    assert_eq!(
        session.snapshot(&roots[..1]).unwrap_err().kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    #[cfg(not(target_arch = "wasm32"))]
    {
        let original: (bool, String, f64, bool, bool) = mlua::Lua::new().load(r#"
            local base = {Record = function(self,value) self.last=value; return 'base' end}
            local class = {Record=base.Record}; class.__index=class
            local object = setmetatable({last=5,savedMethod=base.Record,[base.Record]='method key'},class)
            local saved=object
            local function live(value) return object:Record(value) end
            live(42)
            return object.Record==object.savedMethod, object[object.Record], saved.last,
                rawget(object,'Object')==nil, rawget(object,'_parentInit')==nil
        "#).eval().unwrap();
        assert_eq!(original, (true, "method key".into(), 42.0, true, true));
    }
}
#[test]
fn class_lookup_target_precedes_argument_override_and_nil_deletion_restores_inheritance() {
    let (owner, library) = expanded();
    let (mut session, roots) = library
        .session_from_input(&input(&owner), ProgramLimits::default())
        .unwrap();
    let replacement = args(
        &mut session,
        vec![ProgramValue::Callback(ParserCallbackId(8))],
    );
    let result = session
        .invoke(
            ParserCallbackId(6),
            &[roots[0].clone(), replacement[0].clone()],
        )
        .unwrap();
    assert_eq!(scalar(&mut session, &result[0]), bytes("base"));
    let last = read(&mut session, &roots[0], bytes("last"));
    assert_eq!(scalar(&mut session, &last), ProgramValue::Number(17.0));
    let result = session.invoke_method(&roots[0], "Record", &[]).unwrap();
    assert_eq!(scalar(&mut session, &result[0]), bytes("override"));
    write(&mut session, &roots[0], "Record", ProgramValue::Nil);
    let result = session.invoke_method(&roots[0], "Record", &[]).unwrap();
    assert_eq!(scalar(&mut session, &result[0]), bytes("base"));
    write(
        &mut session,
        &roots[0],
        "Record",
        ProgramValue::Boolean(false),
    );
    let error = session
        .invoke(
            ParserCallbackId(6),
            &[roots[0].clone(), replacement[0].clone()],
        )
        .unwrap_err();
    assert_eq!(error.kind, ProgramRuntimeErrorKind::Source);
    let result = session.invoke_method(&roots[0], "Record", &[]).unwrap();
    assert_eq!(scalar(&mut session, &result[0]), bytes("override"));
}
#[test]
fn unknown_raw_fields_block_inherited_lookup_but_writes_and_deletion_resolve_coverage() {
    let (owner, library) = expanded();
    for selective in [false, true] {
        let mut observation = input(&owner);
        let coverage = observation.coverage.get_mut(&ProgramTableId(1)).unwrap();
        if selective {
            coverage.inventory = SourceTableInventory::Selective;
        } else {
            coverage
                .unavailable
                .insert(SourceTableKey::Text("Record".into()));
        }
        let (mut session, roots) = library
            .session_from_input(&observation, ProgramLimits::default())
            .unwrap();
        let replacement = args(
            &mut session,
            vec![ProgramValue::Callback(ParserCallbackId(8))],
        );
        let error = session
            .invoke(
                ParserCallbackId(6),
                &[roots[0].clone(), replacement[0].clone()],
            )
            .unwrap_err();
        assert_eq!(error.kind, ProgramRuntimeErrorKind::UnsupportedCapability);
        let last = read(&mut session, &roots[0], bytes("last"));
        assert_eq!(
            scalar(&mut session, &last),
            ProgramValue::Number(5.0),
            "argument was not evaluated"
        );
        write(
            &mut session,
            &roots[0],
            "Record",
            ProgramValue::Callback(ParserCallbackId(8)),
        );
        let result = session.invoke_method(&roots[0], "Record", &[]).unwrap();
        assert_eq!(scalar(&mut session, &result[0]), bytes("override"));
        write(&mut session, &roots[0], "Record", ProgramValue::Nil);
        let result = session.invoke_method(&roots[0], "Record", &[]).unwrap();
        assert_eq!(scalar(&mut session, &result[0]), bytes("base"));
    }
}
#[test]
fn imported_class_calls_distinguish_unknown_and_absent_metamethod_after_arguments() {
    let (base_owner, base_library) = expanded();
    for unknown in [false, true] {
        let mut classes = base_owner.classes().unwrap().clone();
        if unknown {
            classes.classes[1]
                .unsupported_fields
                .insert("__call".into());
        }
        let owner = SourceProgramOwner::new_with_closures(
            base_owner.definitions().unwrap().clone(),
            Some(classes),
            None,
            base_owner.closure_prototypes().unwrap().clone(),
        )
        .unwrap();
        let library = rebuild(owner.clone(), &base_library);
        let mut observation = input(&owner);
        observation
            .coverage
            .get_mut(&ProgramTableId(1))
            .unwrap()
            .call_fallback = if unknown {
            SourceTableCallFallback::Unavailable
        } else {
            SourceTableCallFallback::NonCallable
        };
        let (mut session, roots) = library
            .session_from_input(&observation, ProgramLimits::default())
            .unwrap();
        let args = args(
            &mut session,
            vec![
                ProgramValue::Callback(ParserCallbackId(5)),
                ProgramValue::Callback(ParserCallbackId(8)),
            ],
        );
        let error = session
            .invoke(
                ParserCallbackId(19),
                &[roots[0].clone(), args[0].clone(), args[1].clone()],
            )
            .unwrap_err();
        assert_eq!(
            error.kind,
            if unknown {
                ProgramRuntimeErrorKind::UnsupportedCapability
            } else {
                ProgramRuntimeErrorKind::Source
            }
        );
        let changed = read(&mut session, &roots[0], bytes("mutated"));
        assert_eq!(scalar(&mut session, &changed), ProgramValue::Number(1.0));
        observation
            .coverage
            .get_mut(&ProgramTableId(1))
            .unwrap()
            .call_fallback = if unknown {
            SourceTableCallFallback::NonCallable
        } else {
            SourceTableCallFallback::Unavailable
        };
        assert_eq!(
            session.import_session_input(&observation).unwrap_err().kind,
            ProgramRuntimeErrorKind::InvalidInput
        );
    }
}
#[test]
fn class_imports_are_scoped_owner_bound_atomic_and_unavailable_to_plain_inputs() {
    let (owner, library) = expanded();
    let (other, _) = expanded();
    let observation = input(&owner);
    let (mut session, roots) = library
        .session_from_input(&observation, ProgramLimits::default())
        .unwrap();
    let second = session.import_session_input(&observation).unwrap();
    write(&mut session, &second[0], "last", ProgramValue::Number(99.0));
    let old = read(&mut session, &roots[0], bytes("last"));
    assert_eq!(scalar(&mut session, &old), ProgramValue::Number(5.0));
    for change in 0..5 {
        let mut malformed = observation.clone();
        match change {
            0 => {
                malformed.class_bindings.insert(
                    ProgramTableId(1),
                    other.bind_class(SourceClassId(2)).unwrap(),
                );
            }
            1 => {
                malformed.class_bindings.clear();
            }
            2 => {
                malformed
                    .coverage
                    .get_mut(&ProgramTableId(1))
                    .unwrap()
                    .index_fallback = SourceTableIndexFallback::Nil;
            }
            3 => {
                malformed.class_bindings.insert(
                    ProgramTableId(2),
                    owner.bind_class(SourceClassId(2)).unwrap(),
                );
            }
            4 => {
                malformed.state.tables[0]
                    .entries
                    .push((bytes("last"), ProgramValue::Number(7.0)));
            }
            _ => unreachable!(),
        }
        let before = session.allocations();
        assert!(session.import_session_input(&malformed).is_err());
        assert!(session.allocations().values > before.values);
        let value = read(&mut session, &roots[0], bytes("last"));
        assert_eq!(scalar(&mut session, &value), ProgramValue::Number(5.0));
    }
    let plain = ProgramValueGraph {
        values: vec![ProgramValue::Table(ProgramTableId(1))],
        tables: vec![ProgramTable::default()],
    };
    for writable in [false, true] {
        let error = if writable {
            session.import_with_coverage(&plain, &observation.coverage)
        } else {
            session.borrow_with_coverage(&plain, &observation.coverage)
        }
        .unwrap_err();
        assert_eq!(error.kind, ProgramRuntimeErrorKind::InvalidInput);
    }
    let (mut foreign, _) = library
        .session_from_input(&observation, ProgramLimits::default())
        .unwrap();
    assert_eq!(
        foreign
            .invoke_method(&roots[0], "Record", &[])
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::InvalidInput
    );
    let error = match library.session_from_input(
        &observation,
        ProgramLimits {
            max_tables: 0,
            ..ProgramLimits::default()
        },
    ) {
        Ok(_) => panic!("table limit"),
        Err(error) => error,
    };
    assert_eq!(error.kind, ProgramRuntimeErrorKind::ResourceBound);
}
#[test]
fn imported_class_admission_checks_lookup_protocol_without_replaying_constructor_prerequisites() {
    let (base, library) = expanded();
    for failure in [None, Some("__index"), Some("__newindex"), Some("__eq")] {
        let mut defs = base.definitions().unwrap().clone();
        let mut classes = base.classes().unwrap().clone();
        // This is a captured, already constructed instance. A later fresh
        // allocation would still require the constructor's warmed metadata.
        defs.tables[1].fields.remove("_unconstructedMeta");
        defs.tables[1].fields.remove("_constructorInitialised");
        if let Some(name) = failure {
            if name == "__index" {
                defs.tables[1].fields.remove(name);
            } else {
                classes.classes[1].unsupported_fields.insert(name.into());
            }
        }
        let owner = SourceProgramOwner::new_with_closures(
            defs,
            Some(classes),
            None,
            base.closure_prototypes().unwrap().clone(),
        )
        .unwrap();
        let compiled = rebuild(owner.clone(), &library);
        let result = compiled.session_from_input(&input(&owner), ProgramLimits::default());
        if failure.is_some() {
            assert_eq!(
                result.err().unwrap().kind,
                ProgramRuntimeErrorKind::UnsupportedCapability
            );
        } else {
            let (mut session, roots) = result.unwrap();
            let result = session.invoke_method(&roots[0], "Record", &[]).unwrap();
            assert_eq!(scalar(&mut session, &result[0]), bytes("base"));
            assert_eq!(
                session
                    .allocate_instance(&owner.bind_class(SourceClassId(2)).unwrap())
                    .unwrap_err()
                    .kind,
                ProgramRuntimeErrorKind::UnsupportedCapability
            );
        }
    }
}
