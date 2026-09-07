//! Fixed-input native API throughput, with explicit preparation and document modes.
use poe_optimizer_core::{
    evaluation::{
        BackendIdentity, Engine, EvaluationBudget, EvaluationEngine, EvaluationErrorKind,
        EvaluationRequest, EvaluationResult,
    },
    metrics::MetricMeasurement,
    options::EvaluationOptions,
};
use poe_optimizer_native::{NativeBackend, PreparedEvaluation};
use rayon::prelude::*;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    error::Error,
    hint::black_box,
    io::{self, Write},
    path::PathBuf,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, clap::ValueEnum, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Mode {
    Prepared,
    Document,
}

#[derive(clap::Args)]
pub(crate) struct Args {
    /// Supported native build XML or import string. Every iteration uses this fixed input.
    input: PathBuf,
    /// Local Rayon worker limit, 1..64.
    #[arg(long, default_value_t = 1)]
    jobs: usize,
    /// Maximum counted API calls, 1..1,000,000. No uncounted calculation warmup occurs.
    #[arg(long, default_value_t = 10_000)]
    evaluations: usize,
    /// Shared deadline covering input/preparation and all iteration work.
    #[arg(long, default_value_t = 30)]
    timeout_seconds: u64,
    /// Prepared reuses validated inputs; document repeats XML parsing and engine validation.
    #[arg(long, value_enum, default_value_t = Mode::Prepared)]
    mode: Mode,
    #[arg(long)]
    output: Option<PathBuf>,
}

#[derive(Default)]
struct Counts {
    attempts: AtomicUsize,
    completed: AtomicUsize,
    failures: AtomicUsize,
    discarded_late: AtomicUsize,
    deadline_limited: AtomicBool,
}
#[derive(Default)]
struct Observation {
    finite_results: usize,
    non_finite_or_unavailable_results: usize,
    first_digest: Option<[u8; 32]>,
    checksum_words: [u64; 4],
    differing_metrics: bool,
    identity_changed: bool,
    sample: Option<Vec<MetricMeasurement>>,
    errors: BTreeSet<String>,
}
impl Observation {
    fn record_digest(&mut self, digest: [u8; 32]) {
        self.finite_results += 1;
        if let Some(first) = self.first_digest {
            self.differing_metrics |= first != digest;
        } else {
            self.first_digest = Some(digest);
        }
        for (word, bytes) in self.checksum_words.iter_mut().zip(digest.chunks_exact(8)) {
            *word = word.wrapping_add(u64::from_le_bytes(
                bytes.try_into().expect("eight-byte digest chunk"),
            ));
        }
    }
    fn merge(&mut self, other: Self) {
        self.finite_results += other.finite_results;
        self.non_finite_or_unavailable_results += other.non_finite_or_unavailable_results;
        self.differing_metrics |= other.differing_metrics;
        self.identity_changed |= other.identity_changed;
        if let (Some(first), Some(next)) = (self.first_digest, other.first_digest) {
            self.differing_metrics |= first != next;
        } else if self.first_digest.is_none() {
            self.first_digest = other.first_digest;
        }
        for (word, next) in self.checksum_words.iter_mut().zip(other.checksum_words) {
            *word = word.wrapping_add(next);
        }
        if self.sample.is_none() {
            self.sample = other.sample;
        }
        self.errors.extend(other.errors);
        while self.errors.len() > 8 {
            self.errors.pop_last();
        }
    }
}

pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    if !(1..=64).contains(&args.jobs)
        || !(1..=1_000_000).contains(&args.evaluations)
        || args.timeout_seconds == 0
    {
        return Err(
            "Benchmark requires jobs 1..64, evaluations 1..1,000,000 and a positive timeout".into(),
        );
    }
    if let Some(path) = &args.output {
        if path.exists() {
            return Err(format!("Output already exists: {}", path.display()).into());
        }
        super::destination_identity(path)?;
    }
    let started = Instant::now();
    let deadline = started
        .checked_add(Duration::from_secs(args.timeout_seconds))
        .ok_or("Benchmark timeout is too large")?;
    let imported = poe_optimizer_import::decode_build(&super::read_input(&args.input)?)?;
    let request = EvaluationRequest {
        build: poe_optimizer_core::evaluation::BuildDocument {
            format: poe_optimizer_core::evaluation::BuildFormat::PathOfBuilding2Xml,
            content: imported.xml,
        },
        options: EvaluationOptions::default(),
        metrics: Vec::new(),
    };
    let backend = NativeBackend::new();
    // Validate the fixed profile before dispatch even in document mode. Document
    // iterations still parse/validate independently; no calculation is performed here.
    let prepared = backend.prepare(black_box(&request))?;
    let identity = poe_optimizer_native::backend_identity();
    let engine = Engine::new(NativeBackend::new());
    let resolved_jobs = args.jobs.min(args.evaluations);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(resolved_jobs)
        .build()?;
    let preparation_ms = started.elapsed().as_secs_f64() * 1000.0;
    let iteration_started = Instant::now();
    let counts = Counts::default();
    let observations: Vec<_> = pool.install(|| {
        (0..resolved_jobs)
            .into_par_iter()
            .map(|_| {
                iterate(
                    args.mode,
                    args.evaluations,
                    deadline,
                    &counts,
                    &backend,
                    &prepared,
                    &engine,
                    &request,
                    &identity,
                )
            })
            .collect()
    });
    let iteration_seconds = iteration_started.elapsed().as_secs_f64();
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    let mut observed = Observation::default();
    for observation in observations {
        observed.merge(observation);
    }
    let attempts = counts.attempts.load(Ordering::Relaxed);
    let completed = counts.completed.load(Ordering::Relaxed);
    let failures = counts.failures.load(Ordering::Relaxed);
    let discarded_late = counts.discarded_late.load(Ordering::Relaxed);
    debug_assert_eq!(attempts, completed + failures + discarded_late);
    let all_requested_attempted = attempts == args.evaluations;
    let deadline_limited =
        counts.deadline_limited.load(Ordering::Relaxed) && completed < args.evaluations;
    let termination = if !all_requested_attempted || discarded_late > 0 || deadline_limited {
        "time_budget"
    } else {
        "evaluation_limit"
    };
    let status = if !all_requested_attempted || discarded_late > 0 || deadline_limited {
        "partial"
    } else if failures > 0 || observed.identity_changed || observed.differing_metrics {
        "completed_with_errors"
    } else {
        "completed"
    };
    let checksum = if observed.finite_results == completed
        && completed > 0
        && observed.non_finite_or_unavailable_results == 0
    {
        Some(serde_json::json!({
            "schema_version":1,
            "per_result_algorithm":"sha256_sorted_query_unit_schema_and_finite_f64_bits_v1",
            "aggregate_algorithm":"four_independent_wrapping_u64_little_endian_sums",
            "finite_results":observed.finite_results,
            "per_result_sha256":observed.first_digest.map(hex),
            "aggregate_word_sum_hex":observed.checksum_words.iter().map(|word|format!("{word:016x}")).collect::<String>(),
            "all_results_identical":!observed.differing_metrics,
        }))
    } else {
        None
    };
    let report = serde_json::json!({
        "schema_version":1, "status":status, "termination":termination,
        "benchmark":"fixed_input_native_typed_evaluation", "mode":args.mode,
        "input_xml_sha256":imported.sha256, "input_xml_bytes":request.build.content.len(), "source_format":imported.format,
        "backend":identity,
        "budget":{"evaluations":args.evaluations,"timeout_seconds":args.timeout_seconds,"requested_jobs":args.jobs,"resolved_jobs":resolved_jobs},
        "preparation":{"elapsed_ms":preparation_ms,"profile_preparations":1,"calculation_warmups":0,"includes":"input_decode_profile_validation_backend_identity_and_local_pool_creation"},
        "iterations":{"elapsed_ms":iteration_seconds*1000.0,"attempts":attempts,"completed":completed,"failures":failures,"discarded_late":discarded_late,
            "completed_per_second":if iteration_seconds>0.0 {Some(completed as f64/iteration_seconds)} else {None},
            "attempted_per_second":if iteration_seconds>0.0 {Some(attempts as f64/iteration_seconds)} else {None}},
        "elapsed_ms":elapsed_ms, "metric_checksum":checksum, "sample_measurements":observed.sample,
        "non_finite_or_unavailable_completed_results":observed.non_finite_or_unavailable_results,"backend_identity_changed":observed.identity_changed,"errors":observed.errors,
        "measurement_scope": match args.mode {
            Mode::Prepared=>"Each iteration recomputes the supported native pipeline and builds/validates the complete typed result, including XML export and diagnostics; immutable profile parsing is reused.",
            Mode::Document=>"Each iteration reparses the complete XML request, applies the core engine contract, recomputes the supported native pipeline and builds/validates the complete typed result, including XML export and diagnostics.",
        },
        "timing_notes":["Iteration timing includes full typed result construction/validation, Rayon scheduling, shared accounting and finite-metric checksum observation. Preparation is reported separately; elapsed timers stop at worker join, excluding report aggregation, serialization and output.",
            "The shared deadline is cooperative during native calls and input preparation. Completed results arriving after it are discarded; all Rayon workers are joined.",
            "This is fixed-input API throughput for the declared native profile, not optimizer quality or a comparison with Path of Building."],
        "calculation_results_cached":false,
    });
    let mut bytes = serde_json::to_vec_pretty(&report)?;
    bytes.push(b'\n');
    if let Some(path) = &args.output {
        super::write_new(path, &bytes)?;
    } else {
        io::stdout().write_all(&bytes)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn iterate(
    mode: Mode,
    limit: usize,
    deadline: Instant,
    counts: &Counts,
    backend: &NativeBackend,
    prepared: &PreparedEvaluation,
    engine: &Engine<NativeBackend>,
    request: &EvaluationRequest,
    identity: &BackendIdentity,
) -> Observation {
    let mut observed = Observation::default();
    loop {
        let remaining_ms = deadline
            .saturating_duration_since(Instant::now())
            .as_millis();
        if remaining_ms == 0 {
            counts.deadline_limited.store(true, Ordering::Relaxed);
            break;
        }
        if counts
            .attempts
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                (value < limit).then_some(value + 1)
            })
            .is_err()
        {
            break;
        }
        let budget = EvaluationBudget {
            timeout_ms: remaining_ms.min(u64::MAX as u128) as u64,
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match mode {
            Mode::Prepared => backend.evaluate_prepared(black_box(prepared), budget),
            Mode::Document => engine.evaluate(black_box(request), budget),
        }));
        let result = match result {
            Ok(Ok(result)) => black_box(result),
            Ok(Err(error)) => {
                if error.kind == EvaluationErrorKind::Timeout {
                    counts.deadline_limited.store(true, Ordering::Relaxed);
                }
                counts.failures.fetch_add(1, Ordering::Relaxed);
                if observed.errors.len() < 8 {
                    observed.errors.insert(error.to_string());
                }
                continue;
            }
            Err(_) => {
                counts.failures.fetch_add(1, Ordering::Relaxed);
                if observed.errors.len() < 8 {
                    observed.errors.insert("Native evaluation panicked".into());
                }
                continue;
            }
        };
        let digest = finite_digest(&result);
        if Instant::now() >= deadline {
            counts.deadline_limited.store(true, Ordering::Relaxed);
            counts.discarded_late.fetch_add(1, Ordering::Relaxed);
            continue;
        }
        counts.completed.fetch_add(1, Ordering::Relaxed);
        observed.identity_changed |= !same_identity(&result.backend, identity);
        if let Some(digest) = digest {
            observed.record_digest(digest);
        } else {
            observed.non_finite_or_unavailable_results += 1;
        }
        if observed.sample.is_none() {
            observed.sample = Some(result.measurements.clone());
        }
    }
    observed
}
fn same_identity(left: &BackendIdentity, right: &BackendIdentity) -> bool {
    left.id == right.id
        && left.implementation_version == right.implementation_version
        && left.rules_revision == right.rules_revision
        && left.source_fingerprint == right.source_fingerprint
        && left.adapter_fingerprint == right.adapter_fingerprint
}
fn finite_digest(result: &EvaluationResult) -> Option<[u8; 32]> {
    let mut measurements: Vec<_> = result.measurements.iter().collect();
    measurements.sort_by(|left, right| left.query.cmp(&right.query));
    if measurements.is_empty() {
        return None;
    }
    let mut digest = Sha256::new();
    digest.update(b"native-benchmark-finite-metrics-v1");
    digest.update((measurements.len() as u64).to_le_bytes());
    for measurement in measurements {
        let value = measurement
            .value
            .finite()
            .filter(|value| value.is_finite())?;
        let metadata = serde_json::to_vec(&(
            &measurement.query,
            &measurement.unit,
            measurement.schema_version,
        ))
        .ok()?;
        digest.update((metadata.len() as u64).to_le_bytes());
        digest.update(metadata);
        digest.update(value.to_bits().to_le_bytes());
    }
    Some(digest.finalize().into())
}
fn hex(value: [u8; 32]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}
