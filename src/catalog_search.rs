//! Diagnostic finite comparison of caller-supplied complete build documents.
//! Exact request identity is separate from PoB's realized state and game legality.
use poe_optimizer_core::{
    evaluation::{
        BackendIdentity, BuildDocument, BuildFormat, Engine, EvaluationBudget, EvaluationEngine,
        EvaluationRequest,
    },
    metrics::MetricQuery,
    objective::{ObjectiveSpec, ScoringPolicy},
    options::EvaluationOptions,
};
use poe_optimizer_pob::backend::PobBackend;
use poe_optimizer_search::{
    CandidateEvaluator, CandidateMeasurements, EvaluationControl, ExecutionKind, SearchBudget,
    SearchDomain, SearchPlan,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{Mutex, atomic::AtomicBool},
    time::Duration,
};
const MAX_ENTRIES: usize = 64;
const MAX_MANIFEST_BYTES: usize = 64 * 1024;
const MAX_CORPUS_XML_BYTES: usize = 32 * 1024 * 1024;
#[derive(clap::Args)]
pub(crate) struct Args {
    /// Required manifest of complete caller-supplied build alternatives.
    #[arg(long)]
    catalog: PathBuf,
    /// Typed scalar objective and constraints evaluated for the imported selections.
    #[arg(long)]
    objective: PathBuf,
    #[arg(long, default_value = "vendor/path-of-building-poe2")]
    pob: PathBuf,
    /// Maximum concurrent isolated calculation workers.
    #[arg(long, default_value_t = 1, value_parser = positive_usize)]
    jobs: usize,
    /// Includes the one attempt reserved for fresh finalist verification; minimum two.
    #[arg(long, default_value_t = 5, value_parser = evaluation_limit)]
    max_evaluations: usize,
    /// Shared search and finalist-verification deadline, excluding input preparation.
    #[arg(long, default_value_t = 60, value_parser = positive_u64)]
    timeout_seconds: u64,
    /// Write the report to a new file instead of stdout.
    #[arg(long)]
    output: Option<PathBuf>,
    /// Save exact source XML only when the best feasible candidate passes fresh verification.
    #[arg(long)]
    export: Option<PathBuf>,
}

fn positive_usize(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .ok()
        .filter(|number| *number > 0)
        .ok_or_else(|| "value must be a positive integer".into())
}
fn evaluation_limit(value: &str) -> Result<usize, String> {
    positive_usize(value).and_then(|number| {
        if number >= 2 {
            Ok(number)
        } else {
            Err("at least two evaluations are required, including fresh verification".into())
        }
    })
}
fn positive_u64(value: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .ok()
        .filter(|number| *number > 0)
        .ok_or_else(|| "value must be a positive integer".into())
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CatalogManifest {
    schema_version: u32,
    builds: Vec<CatalogPath>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CatalogPath {
    id: String,
    path: String,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct DocumentSelection {
    id: String,
    xml_sha256: String,
}
#[derive(Serialize)]
struct SourceMetadata {
    id: String,
    path: PathBuf,
    input_format: poe_optimizer_import::ImportFormat,
    input_sha256: String,
    xml_sha256: String,
    candidate: DocumentSelection,
}
struct SuppliedCatalog {
    manifest: CatalogManifest,
    path: PathBuf,
    sha256: String,
    alternatives: Vec<SourceMetadata>,
    documents: BTreeMap<DocumentSelection, String>,
    decoded_xml_bytes: usize,
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn load_catalog(path: &Path) -> Result<SuppliedCatalog, Box<dyn Error>> {
    let path = path.canonicalize()?;
    let mut bytes = Vec::new();
    std::fs::File::open(&path)?
        .take(MAX_MANIFEST_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err("Catalog manifest exceeds 65536 bytes".into());
    }
    let manifest: CatalogManifest = serde_json::from_slice(&bytes)?;
    if manifest.schema_version != 1 {
        return Err("Catalog schema_version must be 1".into());
    }
    if manifest.builds.is_empty() || manifest.builds.len() > MAX_ENTRIES {
        return Err("Catalog requires 1..64 build entries".into());
    }
    let mut ids = BTreeSet::new();
    for entry in &manifest.builds {
        if entry.id.is_empty()
            || entry.id.len() > 128
            || !entry
                .id
                .bytes()
                .all(|v| v.is_ascii_alphanumeric() || b"._-".contains(&v))
            || !ids.insert(entry.id.clone())
        {
            return Err("Catalog IDs must be distinct, contain 1..128 ASCII letters/digits/._-, and have no whitespace or newlines".into());
        }
        if entry.path.trim().is_empty()
            || entry.path.len() > 4096
            || entry.path.chars().any(char::is_control)
        {
            return Err(
                "Catalog paths must contain 1..4096 bytes and no control characters or newlines"
                    .into(),
            );
        }
    }
    let parent = path.parent().ok_or("Catalog path has no parent")?;
    let mut documents = BTreeMap::new();
    let mut alternatives = Vec::new();
    let mut hashes = BTreeSet::new();
    let mut decoded_xml_bytes = 0usize;
    for entry in &manifest.builds {
        let input_path = parent
            .join(&entry.path)
            .canonicalize()
            .map_err(|error| format!("Catalog entry {} path: {error}", entry.id))?;
        let source = super::read_input(&input_path)
            .map_err(|error| format!("Catalog entry {} input: {error}", entry.id))?;
        let imported = poe_optimizer_import::decode_build(&source)
            .map_err(|error| format!("Catalog entry {} import: {error}", entry.id))?;
        decoded_xml_bytes = decoded_xml_bytes
            .checked_add(imported.xml.len())
            .ok_or("Catalog XML byte count overflow")?;
        if decoded_xml_bytes > MAX_CORPUS_XML_BYTES {
            return Err("Catalog decoded XML exceeds 32 MiB".into());
        }
        if !hashes.insert(imported.sha256.clone()) {
            return Err("Duplicate source XML is not a separate catalog alternative".into());
        }
        let candidate = DocumentSelection {
            id: entry.id.clone(),
            xml_sha256: imported.sha256.clone(),
        };
        alternatives.push(SourceMetadata {
            id: entry.id.clone(),
            path: input_path,
            input_format: imported.format,
            input_sha256: hash(&source),
            xml_sha256: imported.sha256,
            candidate: candidate.clone(),
        });
        documents.insert(candidate, imported.xml);
    }
    Ok(SuppliedCatalog {
        manifest,
        path,
        sha256: hash(&bytes),
        alternatives,
        documents,
        decoded_xml_bytes,
    })
}
impl SuppliedCatalog {
    fn materialize(&self, candidate: &DocumentSelection) -> Result<BuildDocument, String> {
        let content = self
            .documents
            .get(candidate)
            .ok_or("Candidate is not an exact member of the supplied document catalog")?;
        Ok(BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: content.clone(),
        })
    }
}
struct SuppliedDomain<'a> {
    registry: &'a SuppliedCatalog,
}
impl SearchDomain<DocumentSelection> for SuppliedDomain<'_> {
    fn validate(
        &self,
        candidate: &DocumentSelection,
        control: &EvaluationControl<'_>,
    ) -> Result<(), String> {
        if control.should_stop() {
            return Err("Search stopped before catalog membership validation".into());
        }
        if self.registry.documents.contains_key(candidate) {
            Ok(())
        } else {
            Err("Unknown supplied document identity".into())
        }
    }
}
struct CalibrationEvaluator<'a> {
    registry: &'a SuppliedCatalog,
    engine: Engine<PobBackend>,
    metrics: Vec<MetricQuery>,
    identity: Mutex<Option<BackendIdentity>>,
    warnings: Mutex<BTreeSet<String>>,
    realized: Mutex<BTreeMap<DocumentSelection, serde_json::Value>>,
}
impl CandidateEvaluator<DocumentSelection> for CalibrationEvaluator<'_> {
    fn execution_kind(&self) -> ExecutionKind {
        ExecutionKind::ExternalProcess
    }
    fn evaluate(
        &self,
        candidate: &DocumentSelection,
        control: &EvaluationControl<'_>,
    ) -> Result<CandidateMeasurements, String> {
        if control.should_stop() {
            return Err("Search stopped before calculation".into());
        }
        let timeout_ms = control.remaining().min(Duration::from_secs(30)).as_millis() as u64;
        if timeout_ms == 0 {
            return Err("Search deadline has less than one millisecond remaining".into());
        }
        let build = self.registry.materialize(candidate)?;
        // Membership selects the exact decoded request, not a projection of its
        // game state. The backend may normalize it; those observations stay separate.
        let result = self
            .engine
            .evaluate(
                &EvaluationRequest {
                    build,
                    options: EvaluationOptions::default(),
                    metrics: self.metrics.clone(),
                },
                EvaluationBudget { timeout_ms },
            )
            .map_err(|error| error.to_string())?;
        {
            let mut recorded = self
                .identity
                .lock()
                .map_err(|_| "Backend identity lock was poisoned")?;
            if recorded
                .as_ref()
                .is_some_and(|identity| identity != &result.backend)
            {
                return Err(
                    "Backend identity changed during the run; measurements were rejected".into(),
                );
            }
            if recorded.is_none() {
                *recorded = Some(result.backend.clone());
            }
        }
        let mut evidence = serde_json::json!({
            "candidate":candidate,"requested_xml_sha256":candidate.xml_sha256,
            "realized_summary":result.build,"realized_context":result.context,"realized_coverage":result.coverage,
            "realized_exports":result.exports.iter().map(|document|serde_json::json!({"format":document.format,"xml_sha256":hash(document.content.as_bytes()),"matches_requested_xml":hash(document.content.as_bytes())==candidate.xml_sha256})).collect::<Vec<_>>(),
            "generic_realization":"unverified","game_legality":"unverified","diagnostic_only":true
        });
        let mut realized = self
            .realized
            .lock()
            .map_err(|_| "Realized evidence lock was poisoned")?;
        let count = realized
            .get(candidate)
            .and_then(|v| v["successful_evaluations"].as_u64())
            .unwrap_or(0)
            + 1;
        evidence["successful_evaluations"] = count.into();
        // Bounded reporting evidence only: it is never consulted to serve a
        // calculation, nor treated as a generic no-normalization certificate.
        realized.insert(candidate.clone(), evidence);
        drop(realized);
        self.warnings
            .lock()
            .map_err(|_| "Warning lock was poisoned")?
            .extend(result.warnings);
        Ok(CandidateMeasurements {
            measurements: result.measurements,
            diagnostic_only: true,
        })
    }
}
pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    for path in [&args.output, &args.export].into_iter().flatten() {
        if path.exists() {
            return Err(format!("Output already exists: {}", path.display()).into());
        }
    }
    let output_identity = args
        .output
        .as_deref()
        .map(super::destination_identity)
        .transpose()?;
    let export_identity = args
        .export
        .as_deref()
        .map(super::destination_identity)
        .transpose()?;
    if output_identity.is_some() && output_identity == export_identity {
        return Err("JSON and XML outputs need different paths".into());
    }
    let objective = super::read_json::<ObjectiveSpec>(&args.objective, 64 * 1024)?;
    let engine = Engine::new(PobBackend::new(std::env::current_exe()?, args.pob));
    let policy = objective.compile(&engine.capabilities().metrics)?;
    let registry = load_catalog(&args.catalog)?;
    let count = registry.alternatives.len();
    let domain = SuppliedDomain {
        registry: &registry,
    };
    let evaluator = CalibrationEvaluator {
        registry: &registry,
        engine,
        metrics: policy.required_metrics(),
        identity: Mutex::new(None),
        warnings: Mutex::new(BTreeSet::new()),
        realized: Mutex::new(BTreeMap::new()),
    };
    let budget = SearchBudget {
        max_evaluations: args.max_evaluations,
        max_proposals: count,
        max_rounds: 1,
        duration: Duration::from_secs(args.timeout_seconds),
        jobs: args.jobs,
        beam_per_status: count,
        archive_size: count,
        verification_attempts: 1,
        seed: 0,
    };
    let search = poe_optimizer_search::search(
        &domain,
        &evaluator,
        &policy,
        SearchPlan::Finite(
            registry
                .alternatives
                .iter()
                .map(|entry| entry.candidate.clone())
                .collect(),
        ),
        &budget,
        &AtomicBool::new(false),
    )?;
    let best = search.feasible.first().filter(|entry| {
        search.verifications.iter().any(|verification| {
            verification.candidate == entry.candidate && verification.consistent
        })
    });
    let verified=best.map(|entry|serde_json::json!({"alternative_id":entry.candidate.id,"xml_sha256":entry.candidate.xml_sha256,"candidate":entry.candidate,"assessment":entry.assessment,"fresh_numeric_verification":true,"generic_realization":"unverified","game_legality":"unverified","diagnostic_only":true}));
    let export_document = best
        .map(|entry| registry.materialize(&entry.candidate))
        .transpose()?;
    let export_status = if args.export.is_none() {
        serde_json::json!({"status":"not_requested"})
    } else if export_document.is_some() {
        serde_json::json!({"status":"written","format":"path_of_building2_xml","source":"exact_requested_xml","does_not_certify_pob_normalization":true})
    } else {
        serde_json::json!({"status":"not_written","reason":"No feasible best candidate passed fresh numeric verification within the shared budget"})
    };
    let report = serde_json::json!({
        "schema_version":2,"status":"experimental_calibration_search","scope":"supplied_build_catalog","diagnostic_only":true,
        "objective":objective,"candidate_domain":{"kind":"exact_supplied_document_membership","generic_realization":"unverified","game_legality":"unverified"},
        "catalog":{"path":registry.path,"sha256":registry.sha256,"manifest":registry.manifest,"decoded_xml_bytes":registry.decoded_xml_bytes,"maximum_entries":MAX_ENTRIES},
        "alternatives":registry.alternatives,
        "backend":*evaluator.identity.lock().map_err(|_|"Backend identity lock was poisoned")?,
        "warnings":*evaluator.warnings.lock().map_err(|_|"Warning lock was poisoned")?,
        "realized_observations":evaluator.realized.lock().map_err(|_|"Realized evidence lock was poisoned")?.values().collect::<Vec<_>>(),
        "worker_timeout_cap_ms":30000,"scenario":"Imported per-document selections and configuration; no encounter or action overrides",
        "verification_scope":"Fresh finalist numeric consistency under the same backend; generic realization and game legality remain unverified",
        "search":search,"best_verified":verified,"export":export_status
    });
    let mut json = serde_json::to_vec_pretty(&report)?;
    json.push(b'\n');
    if let Some(path) = args.export
        && let Some(document) = export_document
    {
        super::write_new(&path, document.content.as_bytes())?;
    }
    if let Some(path) = args.output {
        super::write_new(&path, &json)?;
    } else {
        io::stdout().write_all(&json)?;
    }
    Ok(())
}
