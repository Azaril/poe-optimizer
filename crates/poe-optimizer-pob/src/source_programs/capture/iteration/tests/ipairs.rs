use super::*;

fn link(owner: &SourceProgramOwner) -> (SourceCallbackId, SourceCallbackId) {
    let links = &owner.iteration().unwrap().ipairs_aux;
    assert_eq!(links.len(), 1);
    let (&factory, &auxiliary) = links.iter().next().unwrap();
    assert_eq!(
        owner.intrinsic(factory),
        Some(SourceProgramIntrinsic::Ipairs)
    );
    assert_eq!(
        owner.intrinsic(auxiliary),
        Some(SourceProgramIntrinsic::IpairsAux)
    );
    assert_eq!(owner.ipairs_aux_callback(factory), Some(auxiliary));
    assert_eq!(
        owner.callback(auxiliary).unwrap().kind,
        SourceCallbackKind::Builtin {
            symbol: "ipairs_aux".into()
        }
    );
    assert!(owner.callback(factory).unwrap().upvalues.is_empty());
    assert!(owner.callback(auxiliary).unwrap().upvalues.is_empty());
    (factory, auxiliary)
}

#[test]
fn enabled_capture_retains_original_factory_auxiliary_and_legacy_bytes_stay_unchanged() {
    let f = fixture("local original=ipairs\nreturn function(t) return original(t) end\n");
    let off = observe(&f, SourceCaptureContext::default()).unwrap();
    let before = serde_json::to_vec(off.owner().definitions().unwrap()).unwrap();
    assert!(off.owner().iteration().is_none());
    assert_eq!(off.owner().callbacks().len(), 2);
    assert!(
        !off.owner()
            .definitions()
            .unwrap()
            .intrinsics
            .values()
            .any(|op| *op == SourceProgramIntrinsic::IpairsAux)
    );
    let on = observe(&f, enabled()).unwrap();
    let (factory, auxiliary) = link(on.owner());
    assert_eq!(on.owner().callbacks().len(), 3);
    assert_eq!(
        on.owner()
            .callback(on.callbacks()["evaluate"])
            .unwrap()
            .upvalues[0]
            .value,
        SourceValue::Callback(factory)
    );
    let table = f.lua.create_table().unwrap();
    let actual: MultiValue = f.function.call(table.clone()).unwrap();
    assert_eq!(actual.len(), 3);
    let Value::Function(iterator) = &actual[0] else {
        panic!("iterator")
    };
    assert_eq!(
        iterator.to_pointer(),
        f.observer.iterator_primitives.ipairs_aux.to_pointer()
    );
    assert!(matches!(&actual[1],Value::Table(value) if value.to_pointer()==table.to_pointer()));
    assert!(matches!(actual[2], Value::Integer(0) | Value::Number(0.0)));
    assert_ne!(factory, auxiliary);
    let lowered = lower_from_sources(&f.sources, on.owner()).unwrap();
    assert!(
        lowered.unsupported().is_empty(),
        "{:?}",
        lowered.unsupported()
    );
    assert_eq!(lowered.catalog().data().programs.len(), 1);
    let again = observe(&f, SourceCaptureContext::default()).unwrap();
    assert_eq!(
        before,
        serde_json::to_vec(again.owner().definitions().unwrap()).unwrap()
    );
}

#[test]
fn direct_auxiliary_captures_share_identity_and_retain_the_actual_factory_edge() {
    let f = fixture(
        "local auxiliary=ipairs({})\nlocal a,b=auxiliary,auxiliary\nreturn function(t,i) return a(t,i),a==b end\n",
    );
    assert!(
        observe(&f, SourceCaptureContext::default())
            .unwrap_err()
            .to_string()
            .contains("no observed primitive identity")
    );
    let observed = observe(&f, enabled()).unwrap();
    let (_, auxiliary) = link(observed.owner());
    let captures = &observed
        .owner()
        .callback(observed.callbacks()["evaluate"])
        .unwrap()
        .upvalues;
    assert_eq!(captures.len(), 2);
    assert!(
        captures
            .iter()
            .all(|capture| capture.value == SourceValue::Callback(auxiliary))
    );
    let lowered = lower_from_sources(&f.sources, observed.owner()).unwrap();
    assert!(
        lowered.unsupported().is_empty(),
        "{:?}",
        lowered.unsupported()
    );
    assert_eq!(lowered.catalog().data().programs.len(), 1);
}

#[test]
fn live_captured_factory_keeps_generic_initializer_and_complete_source_body() {
    let f = fixture(
        "local original=ipairs\nreturn function(t) local sum=0 for k,v in original(t) do sum=sum+v end return sum end\n",
    );
    let observed = f
        .observer
        .observe_session(
            &f.lua,
            &f.sources,
            f.source.clone(),
            SourceSessionCaptureRequest {
                callbacks: [("live".into(), f.function.clone())].into(),
                definitions: enabled(),
                ..SourceSessionCaptureRequest::default()
            },
        )
        .unwrap();
    let (factory, _) = link(observed.owner());
    assert!(
        observed
            .input()
            .cells
            .contains(&SourceSessionValue::Callback(factory))
    );
    let lowered = lower_from_sources(&f.sources, observed.owner()).unwrap();
    assert!(
        lowered.unsupported().is_empty(),
        "{:?}",
        lowered.unsupported()
    );
    assert_eq!(lowered.catalog().data().programs.len(), 1);
    let program = &lowered.catalog().data().programs[0];
    assert!(
        program
            .bindings
            .iter()
            .any(|binding| matches!(binding, SourceProgramBinding::DynamicCall {}))
    );
    assert!(program.body.iter().any(|statement| matches!(
        statement.operation,
        SourceProgramStatementKind::ForEach {
            iterator: SourceProgramIterator::Generic { .. },
            ..
        }
    )));
}

#[test]
fn rebound_globals_do_not_replace_the_retained_nonglobal_auxiliary() {
    let f = fixture(
        "local original=ipairs\nlocal auxiliary=original({})\nipairs_aux=function() return 'replacement' end\nipairs=function() return ipairs_aux end\nreturn function(t)\n local actual,state,index=original(t)\n return actual==auxiliary,actual==ipairs(t),actual==ipairs_aux,state==t,index\nend\n",
    );
    assert!(
        observe(&f, enabled()).is_err(),
        "legacy original-global contract rejects rebinding"
    );
    let globals = f.lua.globals();
    let observed = observe(
        &f,
        SourceCaptureContext {
            capture_iteration: true,
            projections: vec![SourceTableSelection {
                table: globals.clone(),
                fields: ["ipairs".into(), "ipairs_aux".into()].into(),
                indexed: Default::default(),
                allow_index_fallback: false,
                allow_call_fallback: false,
            }],
            environment: Some(SourceEnvironmentSelection {
                table: globals,
                root_name: "globals".into(),
            }),
            source_names: Default::default(),
        },
    )
    .unwrap();
    let (_, auxiliary) = link(observed.owner());
    let environment = observed.owner().bind_environment().unwrap().unwrap();
    assert_ne!(
        environment.table().fields["ipairs_aux"],
        SourceValue::Callback(auxiliary)
    );
    assert_ne!(
        environment.table().fields["ipairs"],
        SourceValue::Callback(auxiliary)
    );
    let lowered = lower_from_sources(&f.sources, observed.owner()).unwrap();
    assert!(
        lowered.unsupported().is_empty(),
        "{:?}",
        lowered.unsupported()
    );
    assert_eq!(lowered.catalog().data().programs.len(), 3);
    let table = f.lua.create_table().unwrap();
    let actual: MultiValue = f.function.call(table).unwrap();
    assert_eq!(actual.len(), 5);
    assert!(matches!(actual[0], Value::Boolean(true)));
    assert!(matches!(actual[1], Value::Boolean(false)));
    assert!(matches!(actual[2], Value::Boolean(false)));
    assert!(matches!(actual[3], Value::Boolean(true)));
    assert!(matches!(actual[4], Value::Integer(0) | Value::Number(0.0)));
}

#[test]
fn changed_original_factory_capture_is_rejected_before_enabled_graph_observation() {
    let f = fixture("return function() return 1 end\n");
    let factory = f.observer.iterator_primitives.ipairs.clone();
    let replacement: Function = f.lua.globals().raw_get("type").unwrap();
    unsafe {
        f.lua
            .exec_raw::<()>((factory.clone(), replacement), |state| {
                assert!(!mlua::ffi::lua_setupvalue(state, 1, 1).is_null());
                mlua::ffi::lua_settop(state, 0);
            })
            .unwrap();
    }
    let rejected = observe(&f, enabled()).unwrap_err().to_string();
    unsafe {
        f.lua
            .exec_raw::<()>(
                (factory, f.observer.iterator_primitives.ipairs_aux.clone()),
                |state| {
                    assert!(!mlua::ffi::lua_setupvalue(state, 1, 1).is_null());
                    mlua::ffi::lua_settop(state, 0);
                },
            )
            .unwrap();
    }
    assert!(
        rejected.contains("retained auxiliary capture changed"),
        "{rejected}"
    );
    observe(&f, enabled()).unwrap();
}

#[test]
fn factory_and_auxiliary_capture_shapes_are_complete_before_source_loading() {
    // Manufacture test-only C closures with a test-owned entry point. These
    // are shape negatives, never substitutes for the original iterator code.
    // The observer never calls them.
    let lua = Lua::new();
    let observer = SourceClosureObserver::capture_before_source(&lua).unwrap();
    let original = &observer.iterator_primitives;
    unsafe extern "C-unwind" fn fixture_only(_state: *mut mlua::ffi::lua_State) -> std::ffi::c_int {
        0
    }
    let duplicate = |values: Vec<Value>| -> Function {
        unsafe {
            lua.exec_raw(MultiValue::from_vec(values), |state| {
                let captures = mlua::ffi::lua_gettop(state);
                mlua::ffi::lua_pushcclosure(state, fixture_only, captures);
            })
            .unwrap()
        }
    };
    let extra = duplicate(vec![
        Value::Function(original.ipairs_aux.clone()),
        Value::Nil,
    ]);
    let mut primitives = Primitives {
        pairs: original.pairs.clone(),
        next: original.next.clone(),
        ipairs: extra,
        ipairs_aux: original.ipairs_aux.clone(),
    };
    assert!(
        primitives
            .verify(&lua)
            .unwrap_err()
            .to_string()
            .contains("retained auxiliary capture changed")
    );
    let hidden = duplicate(vec![Value::Nil]);
    primitives.ipairs = duplicate(vec![Value::Function(hidden.clone())]);
    primitives.ipairs_aux = hidden;
    assert!(
        primitives
            .verify(&lua)
            .unwrap_err()
            .to_string()
            .contains("retained auxiliary capture changed")
    );
    primitives.ipairs_aux = lua.load("return function() end").eval().unwrap();
    primitives.ipairs = duplicate(vec![Value::Function(primitives.ipairs_aux.clone())]);
    assert!(
        primitives
            .verify(&lua)
            .unwrap_err()
            .to_string()
            .contains("retained auxiliary capture changed")
    );
}
