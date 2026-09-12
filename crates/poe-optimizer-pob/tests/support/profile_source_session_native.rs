//! Replay only owned source observations; this module does not invoke Lua.
use super::*;
use history::{Expected, Reference, Step};

#[derive(Clone, Copy, Serialize)]
struct Usage {
    steps: u64,
    pattern_steps: u64,
    values: usize,
    bytes: usize,
    tables: usize,
}
fn usage(session: &ProgramSession) -> Usage {
    let allocation = session.allocations();
    Usage {
        steps: session.steps(),
        pattern_steps: session.pattern_steps(),
        values: allocation.values,
        bytes: allocation.bytes,
        tables: allocation.tables,
    }
}
fn retained(before: Usage, after: Usage) {
    assert!(
        after.steps >= before.steps
            && after.pattern_steps >= before.pattern_steps
            && after.values >= before.values
            && after.bytes >= before.bytes
            && after.tables >= before.tables
    );
}
fn resolve(
    reference: &Reference,
    corpus: &Corpus,
    roots: &[SessionValue],
    inputs: &[SessionValue],
    results: &[Vec<SessionValue>],
) -> SessionValue {
    match reference {
        Reference::Root(name) => roots[corpus.capture.observed.root_index(name).unwrap()].clone(),
        Reference::Input(index) => inputs[*index].clone(),
        Reference::Result(call, index) => results[*call][*index].clone(),
    }
}
fn root(corpus: &Corpus, roots: &[SessionValue], name: &str) -> SessionValue {
    roots[corpus.capture.observed.root_index(name).unwrap()].clone()
}
fn lookup(
    session: &mut ProgramSession,
    corpus: &Corpus,
    roots: &[SessionValue],
    key: &SessionValue,
) -> SessionValue {
    session
        .invoke_callable(
            &root(corpus, roots, "probe.lookup"),
            &[root(corpus, roots, "public.cache"), key.clone()],
        )
        .unwrap()
        .remove(0)
}
fn snapshot(
    phases: &mut Vec<Phase>,
    session: &mut ProgramSession,
    values: &[SessionValue],
    expected: &Json,
    label: &str,
) -> SourceProgramOutput {
    let output = timed(phases, label, || session.snapshot(values).unwrap());
    assert_eq!(observation::canonical(output.graph()), *expected, "{label}");
    output
}

pub fn run(corpus: Corpus) -> Json {
    let requested_before_native = allocations::snapshot();
    let mut phases = Vec::new();
    let compiled = timed(&mut phases, "compile_after_source_host_destruction", || {
        CompiledSourcePrograms::new(corpus.capture.lowered.catalog()).unwrap()
    });
    assert!(
        compiled
            .catalog()
            .is_bound_to(corpus.capture.observed.owner())
    );
    let shared = timed(&mut phases, "compiled_library_arc_clone", || {
        compiled.clone()
    });
    assert!(std::ptr::eq(shared.catalog(), compiled.catalog()));
    timed(&mut phases, "compiled_library_nonfinal_drop", || {
        drop(shared)
    });
    let input_copy = timed(&mut phases, "coherent_input_deep_clone", || {
        corpus.capture.observed.input().clone()
    });
    let original = corpus.capture.observed.input();
    assert!(input_copy.owner.is_same_owner(&original.owner));
    assert_eq!(input_copy.state, original.state);
    assert_eq!(input_copy.cells, original.cells);
    assert_eq!(input_copy.closures.len(), original.closures.len());
    assert!(!std::ptr::eq(
        input_copy.state.tables.as_ptr(),
        original.state.tables.as_ptr()
    ));
    timed(&mut phases, "drop_input_deep_clone_owner_retained", || {
        drop(input_copy)
    });
    let (mut session, roots) = timed(&mut phases, "first_private_session_import", || {
        compiled.session_from_input(original, limits()).unwrap()
    });
    assert!(session.is_bound_to(&original.owner));
    let imported_usage = usage(&session);
    let inputs = timed(&mut phases, "scalar_arguments_import_once", || {
        session.borrow(&corpus.history.inputs).unwrap()
    });
    let handle = timed(&mut phases, "session_handle_alias_clone", || {
        roots[0].clone()
    });
    timed(&mut phases, "session_handle_alias_drop", || drop(handle));
    let mut results: Vec<Vec<SessionValue>> = Vec::new();
    let mut calls = Vec::new();
    let mut first_output = None;
    let mut successes = 0;
    let mut source_errors = 0;
    let mut unsupported = 0;
    for (step_index, step) in corpus.history.steps.iter().enumerate() {
        match step {
            Step::Call {
                name,
                arguments,
                expected,
                source_inner_calls,
            } => {
                let arguments = arguments
                    .iter()
                    .map(|r| resolve(r, &corpus, &roots, &inputs, &results))
                    .collect::<Vec<_>>();
                let callable = root(&corpus, &roots, name);
                let before = usage(&session);
                let result = timed(&mut phases, &format!("invoke/{step_index}/{name}"), || {
                    session.invoke_callable(&callable, &arguments)
                });
                let after = usage(&session);
                retained(before, after);
                let mut source_success = Json::Null;
                let (values, category, error) = match expected {
                    Expected::Return(expected) => {
                        let values =
                            result.unwrap_or_else(|e| panic!("step {step_index} {name}: {e}"));
                        let output = snapshot(
                            &mut phases,
                            &mut session,
                            &values,
                            expected,
                            &format!("snapshot/{step_index}/return"),
                        );
                        if first_output.is_none() {
                            first_output = Some(output);
                        } else {
                            timed(&mut phases, "drop_verified_snapshot", || drop(output));
                        }
                        if *name == "original.parser" {
                            successes += 1;
                        }
                        (values, "success", Json::Null)
                    }
                    Expected::SourceError(source_error) => {
                        let error = result.expect_err("original source-error history");
                        assert_eq!(
                            error.kind,
                            ProgramRuntimeErrorKind::Source,
                            "{source_error}"
                        );
                        assert!(error.callback.is_some() && error.location.is_some());
                        assert!(after.steps > before.steps);
                        source_errors += 1;
                        (
                            Vec::new(),
                            "source_error",
                            json!({"source":source_error,"native":error.message,
                            "callback":error.callback,"location":error.location}),
                        )
                    }
                    Expected::Unsupported(source_result) => {
                        let error = result.expect_err(
                            "positive miss must retain its measured unsupported frontier",
                        );
                        assert_eq!(error.kind, ProgramRuntimeErrorKind::UnsupportedCapability);
                        assert_eq!(error.callback, Some(corpus.copy_callback));
                        assert!(error.location.is_some());
                        assert_eq!(
                            error.message,
                            "current session table traversal order is unavailable"
                        );
                        assert!(after.steps > before.steps);
                        unsupported += 1;
                        source_success = source_result.clone();
                        (
                            Vec::new(),
                            "unsupported_positive_miss",
                            json!({"native":error.message,
                            "callback":error.callback,"location":error.location,
                            "declaration":compiled.catalog().owner().callback(corpus.copy_callback).unwrap().kind}),
                        )
                    }
                };
                calls.push(json!({"step":step_index,"target":name,"outcome":category,
                    "before":before,"after_invocation_before_snapshot":after,
                    "source_inner_calls":source_inner_calls,"error":error,
                    "source_success_not_native_success":source_success}));
                results.push(values);
            }
            Step::Check {
                label,
                values,
                expected,
            } => {
                let values = values
                    .iter()
                    .map(|r| resolve(r, &corpus, &roots, &inputs, &results))
                    .collect::<Vec<_>>();
                let output = snapshot(
                    &mut phases,
                    &mut session,
                    &values,
                    expected,
                    &format!("snapshot/{step_index}/{label}"),
                );
                timed(&mut phases, "drop_verified_checkpoint", || drop(output));
            }
        }
    }
    assert_eq!((successes, source_errors, unsupported), (11, 2, 6));
    let after_history = usage(&session);
    let output = first_output.unwrap(); // Actual initialized cache row, not fabricated data.
    let output_copy = timed(&mut phases, "raw_output_deep_clone", || output.clone());
    assert_eq!(output.graph(), output_copy.graph());
    assert!(output.owner().is_same_owner(output_copy.owner()));
    timed(&mut phases, "drop_raw_output_clone", || drop(output_copy));

    // Selected raw export/import only; no closure/layout/class checkpoint claim.
    let (mut transport, _) = compiled
        .session(&ProgramValueGraph::default(), limits())
        .unwrap();
    let raw_roots = timed(&mut phases, "selected_raw_graph_reimport", || {
        transport
            .import_with_coverage(output.graph(), &ProgramTableCoverage::new())
            .unwrap()
    });
    let roundtrip = snapshot(
        &mut phases,
        &mut transport,
        &raw_roots,
        &corpus.history.baseline_row,
        "selected_raw_graph_reexport",
    );
    timed(&mut phases, "drop_raw_transport_output_and_handles", || {
        drop((roundtrip, raw_roots))
    });
    timed(&mut phases, "drop_raw_transport_session", || {
        drop(transport)
    });
    timed(&mut phases, "drop_retained_initial_snapshot", || {
        drop(output)
    });

    // Mutate the first live cache while the independent session remains original.
    let marker_graph = ProgramValueGraph {
        values: vec![ProgramValue::Number(99.0)],
        tables: Vec::new(),
    };
    let markers = timed(&mut phases, "isolation_marker_import", || {
        session.borrow(&marker_graph).unwrap()
    });
    session
        .invoke_callable(
            &root(&corpus, &roots, "probe.replace"),
            &[
                root(&corpus, &roots, "public.cache"),
                inputs[corpus.history.hit_key].clone(),
                markers[0].clone(),
            ],
        )
        .unwrap();
    let mutated = lookup(
        &mut session,
        &corpus,
        &roots,
        &inputs[corpus.history.hit_key],
    );
    let mutation = session.snapshot(std::slice::from_ref(&mutated)).unwrap();
    assert_eq!(mutation.graph().values, vec![ProgramValue::Number(99.0)]);
    let (mut independent, independent_roots) = timed(
        &mut phases,
        "independent_private_session_import_while_first_mutated",
        || {
            compiled
                .session_from_input(corpus.capture.observed.input(), limits())
                .unwrap()
        },
    );
    assert!(independent.is_bound_to(session.owner()));
    let independent_inputs = independent.borrow(&corpus.history.inputs).unwrap();
    let untouched = lookup(
        &mut independent,
        &corpus,
        &independent_roots,
        &independent_inputs[corpus.history.hit_key],
    );
    let witness = snapshot(
        &mut phases,
        &mut independent,
        std::slice::from_ref(&untouched),
        &corpus.history.baseline_row,
        "independent_original_cache_row",
    );
    let foreign = session
        .invoke_callable(
            &root(&corpus, &roots, "probe.lookup"),
            &[
                root(&corpus, &independent_roots, "public.cache"),
                inputs[corpus.history.hit_key].clone(),
            ],
        )
        .unwrap_err();
    assert_eq!(foreign.kind, ProgramRuntimeErrorKind::InvalidInput);
    timed(&mut phases, "drop_independent_outputs_and_handles", || {
        drop((witness, untouched, independent_inputs, independent_roots))
    });
    timed(
        &mut phases,
        "drop_independent_session_shared_owner_retained",
        || drop(independent),
    );
    let final_usage = usage(&session);
    timed(
        &mut phases,
        "drop_first_session_outputs_and_handles",
        || drop((results, inputs, roots, markers, mutated, mutation)),
    );
    timed(
        &mut phases,
        "drop_first_session_shared_owner_retained",
        || drop(session),
    );

    // There is no reset API. Restart is a fresh coherent import after teardown.
    let (mut restarted, restart_roots) =
        timed(&mut phases, "restart_by_fresh_import_after_drop", || {
            compiled
                .session_from_input(corpus.capture.observed.input(), limits())
                .unwrap()
        });
    let restart_inputs = restarted.borrow(&corpus.history.inputs).unwrap();
    let restart_row = lookup(
        &mut restarted,
        &corpus,
        &restart_roots,
        &restart_inputs[corpus.history.hit_key],
    );
    let restart_output = snapshot(
        &mut phases,
        &mut restarted,
        std::slice::from_ref(&restart_row),
        &corpus.history.baseline_row,
        "fresh_restart_original_cache_row",
    );
    timed(&mut phases, "drop_restart_outputs_and_handles", || {
        drop((restart_output, restart_row, restart_inputs, restart_roots))
    });
    timed(&mut phases, "drop_restart_session", || drop(restarted));
    let identity = corpus.identity;
    let inventory = corpus.capture.inventory;
    timed(
        &mut phases,
        "drop_captured_input_owner_retained_by_library_and_catalog",
        || drop(corpus.capture.observed),
    );
    timed(
        &mut phases,
        "drop_lowered_catalog_owner_retained_by_library",
        || drop(corpus.capture.lowered),
    );
    timed(
        &mut phases,
        "drop_final_compiled_library_and_owned_definitions",
        || drop(compiled),
    );
    let requested_after_native_intervals = allocations::snapshot();
    // Retained workload identification is built only after every timed phase.
    // Result references count calls, whereas step positions also include checks.
    let scalar_inputs = observation::canonical(&corpus.history.inputs);
    assert!(corpus.history.inputs.tables.is_empty());
    let mut call_index = 0;
    let steps = corpus
        .history
        .steps
        .iter()
        .enumerate()
        .map(|(step_index, step)| match step {
            Step::Call {
                name,
                arguments,
                expected,
                source_inner_calls,
            } => {
                let index = call_index;
                call_index += 1;
                let (kind, digest) = match expected {
                    Expected::Return(graph) => ("source_return", json_hash(graph)),
                    Expected::SourceError(error) => ("source_error_text", json_hash(error)),
                    Expected::Unsupported(graph) => {
                        ("source_success_native_unsupported", json_hash(graph))
                    }
                };
                json!({"step":step_index,"call_index":index,"target":name,
                    "arguments":arguments,"source_inner_calls":source_inner_calls,
                    "expected_kind":kind,"expected_sha256":digest})
            }
            Step::Check {
                label,
                values,
                expected,
            } => json!({
                "step":step_index,"checkpoint":label,"references":values,
                "expected_kind":"source_checkpoint","expected_sha256":json_hash(expected)}),
        })
        .collect::<Vec<_>>();
    let history_evidence = json!({
        "scalar_inputs_sha256":json_hash(&scalar_inputs),"scalar_inputs":scalar_inputs,
        "initialized_hit_input_index":corpus.history.hit_key,"steps":steps,
        "hash_encoding":"SHA-256 of serde_json::to_vec bytes: observation::canonical JSON for packs/checkpoints/inputs; JSON string for source-error text",
        "references":"Root names index the captured root map; Input indexes scalar inputs; Result(call_index,value_index) indexes the full result pack of a prior call. Indices are zero-based.",
        "constructed_after_all_timed_phases":true,
    });
    let session_limits = limits();
    json!({"identity":identity,"inventory":inventory,"phases":phases,"calls":calls,
        "requested_layout_process_totals":{"before_native":requested_before_native,
            "after_native_intervals_before_history_report":requested_after_native_intervals},
        "history_evidence":history_evidence,
    "successful_public_calls":successes,"source_error_calls":source_errors,
    "unsupported_positive_misses":unsupported,"imported_usage":imported_usage,
    "after_history_usage":after_history,"final_first_session_usage":final_usage,
    "limits":{"max_steps":session_limits.max_steps,"max_values":session_limits.max_values,
        "max_bytes":session_limits.max_bytes,"max_tables":session_limits.max_tables,
        "pattern_max_steps":session_limits.pattern.max_steps},
    "witnesses":{"exact_owner_retained":true,"source_full_packs_and_raw_prefixes":true,
        "source_alias_and_copy_histories":true,"private_session_isolation":true,
        "foreign_handles_rejected":true,"fresh_restart_restores_original_raw_contents":true,
        "selected_raw_graph_roundtrip_only":true},
    "limitations":[
        "First native compilation/import follows original build, observation and lowering; OS caches and allocator are not cold.",
        "Source host is destroyed before native timing; the measurement executable still links Lua.",
        "Scalar arguments are imported once. Argument vectors, validation, report construction and isolated setup are outside invocation timers.",
        "Checks/snapshots still change allocator reuse and cumulative budgets; timed phases are lifecycle observations, not uninstrumented throughput.",
        "Handles/results are retained for alias checks until explicit teardown; one initial raw snapshot survives until output lifecycle measurements.",
        "VM Usage/allocation fields remain cumulative resource charges. Phase requested_layout fields count successful Rust GlobalAlloc requested layouts forwarded to System; these are different quantities.",
        "Requested-layout accounting is process-wide from startup, not thread-filtered. No native workers are spawned; multi-field snapshots/interval peak resets assume quiescent boundaries, and any concurrent libtest/runtime allocations are included.",
        "Requested layouts exclude RSS, usable allocator sizes, fragmentation, headers, stacks, static data, and Lua/native allocations that bypass Rust GlobalAlloc. This is not a Lua allocator measurement.",
        "A successful realloc counts full new requested bytes and full released old bytes; logical live/peak use their size difference. System's internal temporary storage/copies are unobserved. Null alloc/zeroed/realloc only increment their failed-call counters, never storage traffic or ownership.",
        "Live ownership counters are never reset: releasing earlier allocations gives a negative phase live delta. Absolute phase peaks include the starting process baseline; growth is above that baseline, not an isolated object's peak size.",
        "Interval counts close before Phase name/Vec/report construction. Before/after-native totals include intervening checks, argument construction and existing report storage, and process peak also includes earlier source acquisition.",
        "Atomic allocator hooks and timing/accounting reads add overhead. elapsed_ms is instrumented time, not directly comparable to the previous elapsed-only harness as an uninstrumented throughput measurement.",
        "Compiled library clones share ownership; input/output clones copy raw graphs. ProgramSession has no clone/reset API.",
        "Raw reimport carries no traversal/class/closure checkpoint guarantee; restarts use the original coherent input.",
        "Eleven supported calls and six positive Unsupported misses do not prove a complete native parser or any full native build.",
        "One sequential lifecycle per original; no warmed throughput, parallel speed or numerical winner claim."
    ]})
}
