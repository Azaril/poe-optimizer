//! Compare source activation identities with an opt-in native traversal failure.
use super::*;
use sha2::{Digest, Sha256};

pub(super) fn compare(
    pair: &mut Pair,
    source: &copy_witness::CopyWitness,
    source_row: &Value,
    native_row: &SessionValue,
    witness: &TraversalFailureWitness,
    error: &ProgramRuntimeError,
    allocation_origin: &TableAllocationOrigin,
) -> Json {
    let native_origin = describe_origin(pair, allocation_origin);
    let TableAllocationOrigin::Expression(origin) = allocation_origin else {
        unreachable!("validated native expression origin");
    };
    let distinct = pair
        .call(
            "probe.distinct",
            &[origin.table().clone(), witness.table.clone()],
        )
        .expect("retained allocation handle belongs to the failure session");
    assert!(
        !pair.boolean(&distinct),
        "origin retains the actual failed table identity"
    );
    assert!(witness.owner().is_same_owner(pair.observed.owner()));
    let owner = pair.observed.owner();
    let environment = owner
        .roots()
        .iter()
        .find(|root| root.name == "Environment")
        .unwrap();
    let poe_optimizer_data::source_program::SourceValue::Callback(callback) =
        &owner.table(environment.table).unwrap().fields["copyTable"]
    else {
        panic!("observed immutable copy function binding");
    };
    let callback = *callback;
    // The observer captured this immutable field from the same actual globals
    // and Function retained in Pair and supplied to the source hook. Do not
    // promote it to a live session root or infer identity from its name/span.
    assert_eq!(error.callback, Some(callback));
    assert_eq!(witness.callback, Some(callback));
    assert_eq!(witness.location, error.location);
    let declaration =
        serde_json::to_value(&pair.observed.owner().callback(callback).unwrap().kind).unwrap();
    let function = pair.copy_table.info();
    assert_eq!(
        declaration["source"]["line"].as_u64(),
        function.line_defined.map(|line| line as u64)
    );
    assert_eq!(
        declaration["source"]["end_line"].as_u64(),
        function.last_line_defined.map(|line| line as u64)
    );
    assert_eq!(declaration["source"]["path"], "src/Modules/Common.lua");
    let graph = pair.plain(&[
        native_row.clone(),
        witness.table.clone(),
        witness.control.clone(),
    ]);
    let paths = copy_paths::describe(&graph).unwrap();
    assert_eq!(
        paths["reachable_from_cache"], true,
        "failed input must be tied to this cache row"
    );
    let identity_graph = pair.plain(&[native_row.clone(), witness.table.clone()]);
    let copy_depth = witness
        .frames
        .iter()
        .filter(|frame| frame.callback == callback)
        .count();
    assert!(copy_depth > 0);
    let matching = source
        .activations
        .iter()
        .filter(|activation| {
            let candidate = observation::canonical(&observation::capture(&[
                source_row.clone(),
                activation.table.clone(),
            ]));
            candidate == identity_graph && activation.depth == copy_depth
        })
        .collect::<Vec<_>>();
    assert!(
        !matching.is_empty(),
        "exact source copy input/alias/depth witness missing"
    );
    // Multiple visits to an aliased input remain visible rather than invented as
    // a unique allocation or activation. Current real fixtures can prove uniqueness.
    let activations = matching.iter().map(|activation| {
        let loops=source.loops.iter().filter(|row| row.activation==activation.ordinal).map(|row| {
            assert_eq!(row.table, activation.table);
            json!({"source_line":row.source_line,"control_unavailable_reason":row.control_unavailable_reason,
                "visible_control":row.visible_control.as_ref().map(|v|observation::canonical(&observation::capture(std::slice::from_ref(v)))),
                "visible_key":row.visible_key.as_ref().map(|v|observation::canonical(&observation::capture(std::slice::from_ref(v))))})
        }).collect::<Vec<_>>();
        json!({"ordinal":activation.ordinal,"parent":activation.parent,"depth":activation.depth,
            "parent_key":activation.parent_key.as_ref().map(|v|observation::canonical(&observation::capture(std::slice::from_ref(v)))),
            "no_recurse":observation::canonical(&observation::capture(std::slice::from_ref(&activation.no_recurse))),
            "loop_observations":loops})
    }).collect::<Vec<_>>();
    let frames = witness.frames.iter().map(|frame| json!({"activation":frame.activation,
        "parent_activation":frame.parent_activation,"callback":frame.callback,"location":frame.location,
        "declaration":pair.observed.owner().callback(frame.callback).map(|callback|&callback.kind)})).collect::<Vec<_>>();
    json!({"joint_native_graph":graph,"paths":paths,"joint_input_identity_compared":true,
        "native_frames":frames,"copy_depth":copy_depth,"matching_source_activations":activations,
        "total_source_copy_activations":source.activations.len(),"total_source_loop_observations":source.loops.len(),
        "source_function_identity_checked":true,"native_callback_bound_to_source_function":true,"allocation_origin_proven":false,
        "native_expression_origin":native_origin,"allocation_handle_identity_compared":true,"original_lua_producer_observed":false,
        "scope":"native Next input/control, source copy input identity/alias/depth, and exact native Table-expression origin; original Lua allocation remains unobserved; line-local controls are not intercepted Next arguments; no source order supplied to native; no warm-path claim"})
}

fn describe_origin(pair: &Pair, origin: &TableAllocationOrigin) -> Json {
    let TableAllocationOrigin::Expression(origin) = origin else {
        panic!("positive native parse must retain the actual failed table's allocation");
    };
    assert!(origin.is_bound_to(&pair.compiled));
    let catalog = origin.catalog();
    assert!(std::ptr::eq(catalog.data(), pair.compiled.catalog().data()));
    assert!(catalog.owner().is_same_owner(pair.observed.owner()));
    match (
        catalog.constructors(),
        pair.compiled.catalog().constructors(),
    ) {
        (None, None) => {}
        (Some(actual), Some(expected)) => assert!(std::ptr::eq(actual, expected)),
        _ => panic!("origin must retain the exact constructor facet"),
    }
    let program = catalog
        .for_callback(origin.callback)
        .expect("retained allocating program");
    assert_eq!(program.callback, origin.callback);
    assert!(
        matches!(&catalog.owner().callback(origin.callback).unwrap().kind,
        poe_optimizer_data::source_program::SourceCallbackKind::Lua { source }
            if source == &program.provenance.source)
    );
    let location = serde_json::to_value(origin.location).unwrap();
    // Walk serialized IR only in this oracle. Matching both the exact range and
    // Table operation avoids equating a parent statement with its expression.
    let program_json = serde_json::to_value(program).unwrap();
    let mut pending = vec![&program_json];
    let mut visited = 0usize;
    let mut matches = 0usize;
    while let Some(value) = pending.pop() {
        visited += 1;
        assert!(visited <= 500_000, "allocation-origin IR inspection bound");
        match value {
            Json::Object(fields) => {
                if fields.get("location") == Some(&location)
                    && fields
                        .get("operation")
                        .and_then(|value| value.get("kind"))
                        .and_then(Json::as_str)
                        == Some("table")
                {
                    matches += 1;
                }
                pending.extend(fields.values());
            }
            Json::Array(values) => pending.extend(values),
            _ => {}
        }
    }
    assert_eq!(matches, 1, "exact unique allocating Table expression");

    let sites = catalog
        .constructors()
        .into_iter()
        .flat_map(|facet| &facet.sites)
        .filter(|site| site.callback == origin.callback && site.expression == origin.location)
        .collect::<Vec<_>>();
    assert!(sites.len() <= 1, "unique catalog-bound constructor site");
    let seed_status = if let Some(site) = sites.first() {
        assert_eq!(site.provenance, program.provenance);
        assert!(
            catalog
                .constructors()
                .unwrap()
                .profile
                .is_supported_array_profile(),
            "unsupported-profile constructors fail before allocating a table"
        );
        "admitted_array_seed"
    } else {
        // Absence is not a TDUP observation, unsupported profile, or proof-loss event.
        "no_admitted_constructor_site"
    };

    let vendor =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
    let text =
        poe_optimizer_pob::source::read_verified_text(&vendor, &program.provenance.source.path)
            .unwrap();
    let span = &program.provenance.source;
    let body = text
        .split_inclusive('\n')
        .skip(span.line as usize - 1)
        .take((span.end_line - span.line + 1) as usize)
        .collect::<String>();
    assert_eq!(
        format!("{:x}", Sha256::digest(body.as_bytes())),
        span.sha256
    );
    let function = body
        .get(program.provenance.function_start as usize..program.provenance.function_end as usize)
        .expect("verified function slice within source span");
    assert_eq!(
        format!("{:x}", Sha256::digest(function.as_bytes())),
        program.provenance.function_sha256
    );
    let expression = function
        .get(origin.location.start as usize..origin.location.end as usize)
        .expect("verified Table-expression slice");
    let span_offset = program.provenance.function_start as usize + origin.location.start as usize;
    let line = span.line as usize
        + body.as_bytes()[..span_offset]
            .iter()
            .filter(|byte| **byte == b'\n')
            .count();
    json!({
        "ordinal":origin.ordinal,"callback":origin.callback,"location":origin.location,
        "program_provenance":program.provenance,
        "declaration":catalog.owner().callback(origin.callback).unwrap().kind,
        "source_line":line,"expression_source":expression,
        "seed_status":seed_status,"constructor_site":sites.first(),
        "same_compiled_catalog":true,"exact_table_expression":true,
        "source_function_digest_checked":true,
        "scope":"native allocation identity and retained compiled expression; source text is authenticated but original Lua allocation/store event is not observed; seed presence is not current traversal proof"
    })
}
