//! Compare source activation identities with an opt-in native traversal failure.
use super::*;

pub(super) fn compare(
    pair: &mut Pair,
    source: &copy_witness::CopyWitness,
    source_row: &Value,
    native_row: &SessionValue,
    witness: &TraversalFailureWitness,
    error: &ProgramRuntimeError,
) -> Json {
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
        "scope":"native Next input/control and source copy input identity/alias/depth; line-local controls are not intercepted Next arguments; no source order supplied to native; no warm-path claim"})
}
