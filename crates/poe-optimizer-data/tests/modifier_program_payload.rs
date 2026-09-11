use poe_optimizer_data::{game_data::bundled_package_bytes, modifier_parser::*};
use std::{collections::BTreeMap, sync::OnceLock};

fn definitions() -> ModifierParserData {
    static DATA: OnceLock<ModifierParserData> = OnceLock::new();
    DATA.get_or_init(|| {
        // Authored fixture: retain all real definitions, explicitly clear the
        // packaged programs/admissions before making independent test changes.
        let package: serde_json::Value = serde_json::from_slice(bundled_package_bytes()).unwrap();
        let mut parser = package["modifier_parser"].clone();
        parser["schema_version"] = MODIFIER_PARSER_SCHEMA_VERSION.into();
        parser["programs"] = serde_json::to_value(ParserProgramPayload::default()).unwrap();
        serde_json::from_value(parser).unwrap()
    })
    .clone()
}
fn program(data: &ModifierParserData, id: ParserCallbackId) -> ParserProgram {
    let ParserCallbackKind::Lua { source } = &data.callbacks[id.0 as usize - 1].kind else {
        panic!("Lua fixture")
    };
    ParserProgram {
        callback: id,
        parameter_count: 0,
        variadic: false,
        local_count: 0,
        bindings: vec![],
        body: vec![],
        provenance: ParserProgramProvenance {
            source: source.clone(),
            function_start: 0,
            function_end: 1,
            function_sha256: source.sha256.clone(),
        },
    }
}
fn special(data: &ModifierParserData) -> ParserCallbackId {
    data.tables[data.dictionaries[&ParserDictionary::Special].0 as usize - 1]
        .fields
        .values()
        .find_map(|v| match v {
            ParserValue::Callback(id)
                if matches!(
                    data.factories.get(id),
                    Some(ParserFactoryDisposition::Unsupported { .. })
                ) && matches!(
                    data.callbacks[id.0 as usize - 1].kind,
                    ParserCallbackKind::Lua { .. }
                ) =>
            {
                Some(*id)
            }
            _ => None,
        })
        .unwrap()
}
fn install(data: &mut ModifierParserData, programs: Vec<ParserProgram>) {
    data.programs = ParserProgramPayload {
        data: ParserProgramData {
            schema_version: PARSER_PROGRAM_SCHEMA_VERSION,
            callbacks: programs
                .iter()
                .enumerate()
                .map(|(i, p)| (p.callback, ParserProgramId(i as u32 + 1)))
                .collect(),
            programs,
        },
        admissions: BTreeMap::new(),
    };
}
fn admit(data: &mut ModifierParserData, id: ParserCallbackId, role: ParserProgramRole) {
    let program = &data.programs.data.programs[data.programs.data.callbacks[&id].0 as usize - 1];
    let permission =
        ParserProgramAdmission::bind(data, program, role, "authored-test; no source parity claim")
            .unwrap();
    data.programs.admissions.insert(id, permission);
}
fn admitted() -> (ModifierParserData, ParserCallbackId) {
    let mut data = definitions();
    let id = special(&data);
    let p = program(&data, id);
    install(&mut data, vec![p]);
    admit(&mut data, id, ParserProgramRole::Special);
    (data, id)
}
#[test]
fn empty_payload_and_unadmitted_programs_never_grant_dispatch() {
    let mut data = definitions();
    let id = special(&data);
    let empty = ModifierParserCatalog::new(data.clone()).unwrap();
    let catalog = ParserAdmittedProgramCatalog::new(&empty).unwrap();
    assert!(!catalog.is_admitted(id));
    assert!(!catalog.is_special(id));
    let p = program(&data, id);
    install(&mut data, vec![p]);
    let owner = ModifierParserCatalog::new(data).unwrap();
    let catalog = ParserAdmittedProgramCatalog::new(&owner).unwrap();
    assert!(catalog.programs().for_callback(id).is_some());
    assert!(!catalog.is_admitted(id));
}
#[test]
fn explicit_admission_retains_exact_owner_and_helper_role_is_not_public() {
    let (mut data, id) = admitted();
    let owner = ModifierParserCatalog::new(data.clone()).unwrap();
    let catalog = ParserAdmittedProgramCatalog::new(&owner).unwrap();
    assert!(catalog.is_special(id));
    assert!(catalog.is_admitted(id));
    assert!(catalog.is_bound_to(&owner));
    assert!(!catalog.is_bound_to(&ModifierParserCatalog::new(data.clone()).unwrap()));
    assert_eq!(catalog.owner_sha256(), data.definition_sha256().unwrap());
    data.programs.admissions.get_mut(&id).unwrap().role = ParserProgramRole::Helper;
    let owner = ModifierParserCatalog::new(data).unwrap();
    let helper = ParserAdmittedProgramCatalog::new(&owner).unwrap();
    assert!(helper.is_admitted(id));
    assert!(!helper.is_special(id));
}
#[test]
fn source_ir_lookup_and_capture_mutations_reject_stale_admissions() {
    let (data, id) = admitted();
    let mut changed = data.clone();
    changed
        .programs
        .admissions
        .get_mut(&id)
        .unwrap()
        .source
        .function_sha256 = "0".repeat(64);
    assert!(ModifierParserCatalog::new(changed).is_err());
    let mut changed = data.clone();
    changed.programs.data.programs[0]
        .body
        .push(ParserProgramStatement {
            location: ParserProgramLocation { start: 0, end: 1 },
            operation: ParserProgramStatementKind::Return {
                values: ParserProgramValueList::default(),
            },
        });
    assert!(ModifierParserCatalog::new(changed).is_err());
    let mut changed = data.clone();
    let lookup = changed.dictionaries[&ParserDictionary::GemIdLookup];
    changed.tables[lookup.0 as usize - 1].fields.insert(
        "authored lookup change".into(),
        ParserValue::Text("authored ID".into()),
    );
    assert!(ModifierParserCatalog::new(changed.clone()).is_err());
    // Explicitly rebind changed authored facts; this does not preserve an old proof claim.
    admit(&mut changed, id, ParserProgramRole::Special);
    ModifierParserCatalog::new(changed).unwrap();
    let mut changed = data.clone();
    changed.callbacks[id.0 as usize - 1]
        .upvalues
        .push(ParserUpvalue {
            name: "authored_capture".into(),
            value: ParserValue::Nil,
        });
    assert!(ModifierParserCatalog::new(changed).is_err());
    let mut changed = data;
    changed.policy.doubled_more = -changed.policy.doubled_more;
    assert!(ModifierParserCatalog::new(changed).is_err());
}
#[test]
fn every_captured_program_edge_requires_its_own_admission() {
    let mut data = definitions();
    let (outer, slot, inner) = data
        .callbacks
        .iter()
        .enumerate()
        .find_map(|(i, c)| {
            let outer = ParserCallbackId(i as u32 + 1);
            if !matches!(
                data.factories.get(&outer),
                Some(ParserFactoryDisposition::Unsupported { .. })
            ) {
                return None;
            }
            c.upvalues
                .iter()
                .enumerate()
                .find_map(|(slot, u)| match u.value {
                    ParserValue::Callback(inner)
                        if matches!(
                            data.factories.get(&inner),
                            Some(ParserFactoryDisposition::Unsupported { .. })
                        ) && matches!(
                            data.callbacks[inner.0 as usize - 1].kind,
                            ParserCallbackKind::Lua { .. }
                        ) && inner != outer =>
                    {
                        Some((outer, slot, inner))
                    }
                    _ => None,
                })
        })
        .unwrap();
    let mut p = program(&data, outer);
    p.bindings.push(ParserProgramBinding::CapturedCallback {
        upvalue: slot as u16,
        callback: inner,
    });
    let helper = program(&data, inner);
    install(&mut data, vec![p, helper]);
    admit(&mut data, outer, ParserProgramRole::Helper);
    assert!(ModifierParserCatalog::new(data.clone()).is_err());
    admit(&mut data, inner, ParserProgramRole::Helper);
    let owner = ModifierParserCatalog::new(data).unwrap();
    let catalog = ParserAdmittedProgramCatalog::new(&owner).unwrap();
    assert!(catalog.is_admitted(outer));
    assert!(catalog.is_admitted(inner));
    assert!(!catalog.is_special(outer));
}
#[test]
fn special_roles_require_final_dictionary_membership_and_known_programs() {
    let (mut data, id) = admitted();
    let table = data.dictionaries[&ParserDictionary::Special];
    data.tables[table.0 as usize - 1]
        .fields
        .retain(|_, v| !matches!(v,ParserValue::Callback(c) if *c==id));
    admit(&mut data, id, ParserProgramRole::Special);
    assert!(ModifierParserCatalog::new(data).is_err());
    let (mut data, id) = admitted();
    let record = data.programs.admissions.remove(&id).unwrap();
    data.programs
        .admissions
        .insert(ParserCallbackId(u32::MAX), record);
    assert!(ModifierParserCatalog::new(data).is_err());
}
#[test]
fn payload_serde_rejects_missing_unknown_and_duplicate_fields() {
    let (data, id) = admitted();
    let payload = &data.programs;
    let value = serde_json::to_value(payload).unwrap();
    for key in ["data", "admissions"] {
        let mut incomplete = value.clone();
        incomplete.as_object_mut().unwrap().remove(key);
        assert!(serde_json::from_value::<ParserProgramPayload>(incomplete).is_err());
    }
    let mut unknown = value.clone();
    unknown["permission_for_every_program"] = true.into();
    assert!(serde_json::from_value::<ParserProgramPayload>(unknown).is_err());
    let mut unknown = value;
    unknown["admissions"][id.0.to_string()]["trusted"] = true.into();
    assert!(serde_json::from_value::<ParserProgramPayload>(unknown).is_err());
    let raw_data = serde_json::to_string(&payload.data).unwrap();
    let record = serde_json::to_string(&payload.admissions[&id]).unwrap();
    let duplicate = format!(
        "{{\"data\":{raw_data},\"admissions\":{{\"{}\":{record},\"{}\":{record}}}}}",
        id.0, id.0
    );
    assert!(serde_json::from_str::<ParserProgramPayload>(&duplicate).is_err());
    let mut missing = serde_json::to_value(&data).unwrap();
    missing.as_object_mut().unwrap().remove("programs");
    assert!(serde_json::from_value::<ModifierParserData>(missing).is_err());
    let roundtrip: ModifierParserData =
        serde_json::from_slice(&serde_json::to_vec(&data).unwrap()).unwrap();
    ModifierParserCatalog::new(roundtrip).unwrap();
}
#[test]
fn definition_projection_excludes_only_payload_and_preserves_signed_zero() {
    let (data, id) = admitted();
    let mut expected = serde_json::to_value(&data).unwrap();
    expected.as_object_mut().unwrap().remove("programs");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&data.definition_bytes().unwrap()).unwrap(),
        expected
    );
    let mut empty = data.clone();
    empty.programs = ParserProgramPayload::default();
    assert_eq!(
        empty.definition_bytes().unwrap(),
        data.definition_bytes().unwrap()
    );
    let mut positive = data.clone();
    positive.policy.doubled_more = 0.0;
    let mut negative = positive.clone();
    negative.policy.doubled_more = -0.0;
    assert_ne!(
        positive.definition_sha256().unwrap(),
        negative.definition_sha256().unwrap()
    );
    let mut changed = data.clone();
    changed.programs.admissions.get_mut(&id).unwrap().evidence = "new authored claim".into();
    assert_eq!(
        changed.definition_sha256().unwrap(),
        data.definition_sha256().unwrap()
    );
}
#[test]
fn malformed_incomplete_programs_and_empty_evidence_never_load_as_admitted() {
    let (mut data, id) = admitted();
    data.programs.data.callbacks.clear();
    assert!(ModifierParserCatalog::new(data).is_err());
    let (mut data, id2) = admitted();
    assert_eq!(id, id2);
    data.programs
        .admissions
        .get_mut(&id)
        .unwrap()
        .evidence
        .clear();
    assert!(ModifierParserCatalog::new(data).is_err());
    let (data, _) = admitted();
    assert!(
        ParserProgramAdmission::bind(
            &data,
            &data.programs.data.programs[0],
            ParserProgramRole::Special,
            "x".repeat(4097)
        )
        .is_err()
    );
}
