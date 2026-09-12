//! Compare source activation identities with an opt-in native traversal failure.
use super::*;
use sha2::{Digest, Sha256};

#[allow(clippy::too_many_arguments)]
pub(super) fn compare(
    pair: &mut Pair,
    source: &copy_witness::CopyWitness,
    source_row: &Value,
    native_row: &SessionValue,
    witness: &TraversalFailureWitness,
    error: &ProgramRuntimeError,
    allocation_origin: &TableAllocationOrigin,
    original_constructor: &ConstructorDiagnosticWitness,
    original_tail: &ConstructorDiagnosticWitness,
) -> Json {
    let producer_binding = source
        .producer_binding
        .as_ref()
        .expect("enabled producer observation");
    original_constructor.verify_unchanged().unwrap();
    original_tail.verify_unchanged().unwrap();
    assert_eq!(original_constructor.function(), &producer_binding.target);
    assert_eq!(original_tail.function(), original_constructor.function());
    assert_eq!(original_tail.template(), original_constructor.template());
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
    let producer_stores = source.producer.stores.iter().map(|store| {
        let post_store_proof = producer_proof::post_store(original_constructor.report(), store)
            .expect("exact original post-store continuation and register binding");
        let actual = Value::Table(store.table.clone());
        let matches = matching.iter().filter(|activation| activation.table == actual).map(|activation| {
            assert!(store.observed_event < activation.observed_event, "producer row must predate its copy input");
            activation.ordinal
        }).collect::<Vec<_>>();
        let current: Value = store.list.raw_get(store.index.clone()).unwrap();
        let joint = observation::canonical(&observation::capture(&[
            source_row.clone(), Value::Table(store.list.clone()), actual.clone(), current.clone()]));
        json!({"ordinal":store.ordinal,"observed_event":store.observed_event,"activation":store.activation,
            "source_line":store.source_line,"index":observation::canonical(&observation::capture(std::slice::from_ref(&store.index))),
            "name":observation::canonical(&observation::capture(std::slice::from_ref(&store.name))),
            "local_slots":{"modList":store.list_slot,"i":store.index_slot,"name":store.name_slot},
            "matching_copy_activations":matches,"current_list_slot_is_observed_table":current==actual,
            "post_call_joint_graph":joint,"post_store_proof":post_store_proof})
    }).collect::<Vec<_>>();
    assert!(
        producer_stores
            .iter()
            .any(|store| !store["matching_copy_activations"]
                .as_array()
                .unwrap()
                .is_empty()),
        "an actual retained producer row must join directly to the failed source copy input"
    );
    let producer_activations = source
        .producer
        .activations
        .iter()
        .map(|activation| {
            json!({
        "ordinal":activation.ordinal,"observed_event":activation.observed_event,
        "parent":activation.parent,"depth":activation.depth})
        })
        .collect::<Vec<_>>();
    assert!(
        pair.observed
            .constructor_observations()
            .unwrap()
            .profile()
            .is_supported_array_profile()
    );
    let tail_binding = producer_binding
        .tail
        .as_ref()
        .expect("enabled original tail observation");
    let global_name = original_tail
        .global_name(original_tail.report().continuation_pcs[0], 256)
        .unwrap();
    let tail_observations = source.producer.tails.iter().map(|tail| {
        assert_eq!(tail.caller, producer_binding.target);
        assert_eq!(tail.callee, tail_binding.original_unpack);
        assert_eq!(tail.raw_length, tail.values.len());
        let entry = &source.producer.tail_entries[tail.line_entry_ordinal];
        assert_eq!(entry.ordinal, tail.line_entry_ordinal);
        assert_eq!(entry.activation, tail.activation);
        assert_eq!(entry.caller_depth, tail.caller_depth);
        assert_eq!(entry.source_line, tail.source_line);
        assert_eq!(entry.constructor_slot, tail.constructor_slot);
        assert_eq!(entry.constructor, tail.constructor);
        assert_eq!(entry.original_unpack, tail.callee);
        assert_eq!(entry.consumed_by_tail, Some(tail.ordinal));
        assert!(!entry.abandoned);
        assert!(entry.observed_event < tail.observed_event);

        let proof = producer_proof::tail_call_site(
            original_constructor.report(), original_tail.report(), &global_name, tail.source_line,
            tail.argument_slot, tail.constructor_slot,
        ).expect("exact original single-argument unpack and TSETM dataflow");
        let stores = source.producer.stores.iter().filter(|store|
            store.activation == tail.activation && store.table == tail.constructor
        ).collect::<Vec<_>>();
        assert_eq!(stores.len(), 1, "exact tail constructor must join one observed row store");
        assert!(tail.observed_event < stores[0].observed_event);
        json!({"ordinal":tail.ordinal,"observed_event":tail.observed_event,"activation":tail.activation,
            "caller_depth":tail.caller_depth,"source_line":tail.source_line,
            "argument_local":tail.argument_local,"argument_slot":tail.argument_slot,
            "constructor_slot":tail.constructor_slot,"constructor_local_name":tail.constructor_name,
            "line_entry":{"ordinal":entry.ordinal,"observed_event":entry.observed_event,
                "consumed_by_tail":entry.consumed_by_tail,"abandoned":entry.abandoned,
                "post_call_same_line_events":entry.post_call_same_line_events,
                "plain_environment_and_raw_original_unpack_checked":true,
                "same_environment_checked_at_call":true,"fresh_activation_constructor_token":true},
            "derived_result_count":tail.raw_length,
            "derived_pack":observation::canonical(&observation::capture(&tail.values)),
            "argument_and_constructor_post_call_graph":observation::canonical(&observation::capture(&[
                Value::Table(tail.argument.clone()), Value::Table(tail.constructor.clone())])),
            "matching_store_ordinal":stores[0].ordinal,"actual_caller_and_original_primitive_bound":true,
            "zero_result_tsetm_performs_no_entry_writes_or_array_resize":tail.values.is_empty(),
            "positive_tail_layout_or_start_index_proven":false,"bytecode_dataflow":proof,
            "scope":"raw length and ordered values captured at original unpack call entry; return count/values derived from pinned primitive semantics, not intercepted returns; referenced table contents are post-call"})
    }).collect::<Vec<_>>();
    assert_eq!(tail_observations.len(), source.producer.stores.len());
    assert_eq!(tail_observations.len(), source.producer.tail_entries.len());
    for store in &source.producer.stores {
        assert_eq!(
            source
                .producer
                .tails
                .iter()
                .filter(
                    |tail| tail.activation == store.activation && tail.constructor == store.table
                )
                .count(),
            1,
            "every actual stored constructor has one bound original tail call"
        );
    }
    let producer_observation = json!({"capture_slot":producer_binding.capture_slot,
        "source_line":producer_binding.source_line,"activations":producer_activations,"stores":producer_stores,
        "tail_calls":tail_observations,"tail_line_diagnostic":original_tail.report(),
        "source_runtime_profile":pair.observed.constructor_observations().unwrap().profile(),
        "actual_wrapper_capture_checked_before_and_after":true,"direct_source_table_identity_join":true,
        "contents_timing":"post-call final state of retained source identities; not constructor-completion snapshots",
        "original_post_store_line_region_verified":true,"layout_admitted":false,
        "constructor_diagnostic":original_constructor.report(),"original_template_unchanged_across_call":true});
    let frames = witness.frames.iter().map(|frame| json!({"activation":frame.activation,
        "parent_activation":frame.parent_activation,"callback":frame.callback,"location":frame.location,
        "declaration":pair.observed.owner().callback(frame.callback).map(|callback|&callback.kind)})).collect::<Vec<_>>();
    json!({"joint_native_graph":graph,"paths":paths,"joint_input_identity_compared":true,
        "native_frames":frames,"copy_depth":copy_depth,"matching_source_activations":activations,
        "total_source_copy_activations":source.activations.len(),"total_source_loop_observations":source.loops.len(),
        "source_function_identity_checked":true,"native_callback_bound_to_source_function":true,"allocation_origin_proven":true,
        "original_producer_observation":producer_observation,"native_expression_origin":native_origin,"allocation_handle_identity_compared":true,"original_lua_producer_observed":true,
        "scope":"native allocation and actual original producer/store/copy identity with authenticated constructor instruction/template; no physical layout admission; line-local controls are not intercepted Next arguments; no source order supplied to native; no warm-path claim"})
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
        "scope":"native allocation identity and retained compiled expression; this native witness alone does not observe the original Lua allocation/store event; seed presence is not current traversal proof"
    })
}

pub(super) fn original_sources(pair: &Pair) -> BTreeMap<String, String> {
    let vendor =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
    pair.observed
        .owner()
        .source()
        .files
        .keys()
        .map(|path| {
            let text = if path == PATH {
                TEXT.to_owned()
            } else {
                poe_optimizer_pob::source::read_verified_text(&vendor, path).unwrap()
            };
            (path.clone(), text)
        })
        .collect()
}

pub(super) fn inspect_original_constructor(
    pair: &Pair,
    origin: &TableAllocationOrigin,
    observation_line: u32,
) -> ConstructorDiagnosticWitness {
    let TableAllocationOrigin::Expression(origin) = origin else {
        panic!("actual native origin is required")
    };
    let sources = original_sources(pair);
    let target = pair
        .constructor_target
        .as_ref()
        .expect("pre-observation exact original producer target");
    let observed = target
        .inspect(
            &sources,
            pair.observed.constructor_observations().unwrap(),
            pair.compiled.catalog(),
            ConstructorDiagnosticRequest {
                callback: origin.callback,
                expression: origin.location,
                continuation_line: Some(observation_line),
            },
            ConstructorDiagnosticLimits::default(),
        )
        .unwrap();
    let witness = match observed {
        ConstructorDiagnostic::Observed(witness) => *witness,
        ConstructorDiagnostic::Unavailable(reason) => {
            panic!("exact original constructor unavailable: {reason}")
        }
    };
    assert_eq!(witness.report().callback, origin.callback);
    assert_eq!(witness.report().expression, origin.location);
    assert_eq!(
        witness.report().provenance,
        origin
            .catalog()
            .for_callback(origin.callback)
            .unwrap()
            .provenance
    );
    assert!(std::ptr::eq(
        witness.catalog().data(),
        origin.catalog().data()
    ));
    witness.verify_unchanged().unwrap();
    witness
}
