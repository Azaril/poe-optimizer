//! Host I/O for fresh normalization; all semantic work belongs to the importer.
use poe_optimizer_core::{build_identity::BuildLineage, owned_draft::encode_draft};
use poe_optimizer_data::owned_schema::{OwnedSchemaLimits, decode_schema_package};
use poe_optimizer_import::{
    MAX_XML_BYTES,
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_mapping::{decode_mapping_package, decode_registry},
    owned_normalize::{
        ImportQueryTemplate, NormalizationArtifacts, NormalizationLimits, NormalizationPolicy,
        normalize_fresh,
    },
    owned_skill_catalog::{OwnedSkillRoleIndex, OwnedSkillRolePackageInput, SkillCatalogLimits},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use std::{
    collections::BTreeMap,
    error::Error,
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

#[derive(clap::Args)]
pub(crate) struct Args {
    /// A PoB export XML file or share code, read as source evidence only.
    input: PathBuf,
    /// Explicit normalization policy JSON; no game defaults are supplied.
    #[arg(long)]
    policy: PathBuf,
    /// Persisted owned definition ID registry JSON.
    #[arg(long)]
    registry: PathBuf,
    /// Owned definition schema package JSON.
    #[arg(long)]
    definitions: PathBuf,
    /// Exact external-to-owned mapping package JSON.
    #[arg(long)]
    mapping: PathBuf,
    /// Bound owned skill role package JSON.
    #[arg(long)]
    roles: PathBuf,
    /// Ordered JSON array of explicit import query templates.
    #[arg(long)]
    queries: PathBuf,
    /// New directory for draft.json, sidecar.json and report.json.
    #[arg(long)]
    output: PathBuf,
}

fn read_bounded(path: &Path, maximum: usize) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(maximum as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > maximum {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} exceeds {maximum} bytes", path.display()),
        ));
    }
    Ok(bytes)
}

pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let limits = NormalizationLimits::default();
    let schema_limits = OwnedSchemaLimits::default();
    let registry = decode_registry(
        &read_bounded(&args.registry, limits.mapping.max_wire_bytes)?,
        limits.mapping,
    )?;
    let definitions = decode_schema_package(
        &read_bounded(&args.definitions, schema_limits.max_wire_bytes)?,
        schema_limits,
    )?;
    let mappings = decode_mapping_package(
        &read_bounded(&args.mapping, limits.mapping.max_wire_bytes)?,
        &registry,
        &definitions,
        limits.mapping,
    )?;
    // Deserialize strict DTOs directly; a Value intermediary would hide duplicates.
    let role_input: OwnedSkillRolePackageInput =
        serde_json::from_slice(&read_bounded(&args.roles, limits.mapping.max_wire_bytes)?)?;
    let roles = OwnedSkillRoleIndex::new(
        role_input,
        &mappings,
        &definitions,
        SkillCatalogLimits {
            mapping: limits.mapping,
        },
    )?;
    let policy: NormalizationPolicy =
        serde_json::from_slice(&read_bounded(&args.policy, limits.max_policy_bytes)?)?;
    let queries: Vec<ImportQueryTemplate> =
        serde_json::from_slice(&read_bounded(&args.queries, limits.max_policy_bytes)?)?;
    let decoded = decode_build(&read_bounded(&args.input, MAX_XML_BYTES)?)?;
    let mut lineage = [0u8; 16];
    getrandom::fill(&mut lineage)?;
    let source = ImportedBuildInstance::from_decoded(
        decoded,
        BuildLineage::from_bytes(lineage),
        InstanceImportLimits::default(),
    )?;
    let evidence = SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default())?;
    let normalized = normalize_fresh(
        &evidence,
        *source.allocator_state(),
        NormalizationArtifacts {
            mappings: &mappings,
            registry: &registry,
            definitions: &definitions,
            roles: &roles,
        },
        &policy,
        &queries,
        limits,
    )?;
    let validation = normalized.draft().validate_limits(limits.draft)?;
    let mut issue_codes = BTreeMap::new();
    for issue in &validation.issues {
        *issue_codes.entry(issue.code.as_str()).or_insert(0usize) += 1;
    }
    let draft = encode_draft(normalized.draft(), limits.draft)?;
    let sidecar = serde_json::to_vec(normalized.sidecar())?;
    let input = normalized.draft().input();
    let output = super::destination(&args.output)?;
    let report = serde_json::json!({
        "schema_version": 1,
        "document_kind": "owned_normalization_report",
        "normalization_status": if validation.issues.is_empty() { "no_pending_issues" } else { "pending" },
        "issue_count": validation.issues.len(),
        "issue_codes": issue_codes,
        "source": {
            "sha256": source.source_sha256(),
            "bytes": source.source_xml().len(),
            "occurrences": evidence.rows().len(),
            "origin_rows": normalized.sidecar().origins.len()
        },
        "draft_digest": normalized.sidecar().draft,
        "allocator_before": normalized.sidecar().allocator_before,
        "allocator_after": normalized.allocator_after(),
        "counts": {
            "items": input.items.members.len(),
            "equipment": input.equipment.members.len(),
            "gems": input.gems.members.len(),
            "skills": input.skills.members.len(),
            "supports": input.supports.members.len(),
            "allocations": input.allocations.members.len(),
            "character_presets": input.character_presets.members.len(),
            "equipment_presets": input.equipment_presets.members.len(),
            "allocation_presets": input.allocation_presets.members.len(),
            "skill_presets": input.skill_presets.members.len(),
            "choice_presets": input.choice_presets.members.len(),
            "scenario_presets": input.scenario_presets.members.len(),
            "query_presets": input.query_presets.members.len()
        },
        "output": {
            "directory": output,
            "draft": "draft.json",
            "sidecar": "sidecar.json",
            "report": "report.json"
        },
        "verification": {
            "structure": "valid",
            "artifact_bindings": "checked",
            "definitions": "not_bound",
            "legality": "not_checked",
            "calculation": "not_run"
        }
    });
    let report = serde_json::to_vec_pretty(&report)?;
    // Validation and serialization finish before the output directory is created.
    // create_dir rejects existing directories; each file also uses no-clobber I/O.
    fs::create_dir(&output)?;
    super::write_new(&output.join("draft.json"), &draft)?;
    super::write_new(&output.join("sidecar.json"), &sidecar)?;
    super::write_new(&output.join("report.json"), &report)?;
    let mut stdout = io::stdout().lock();
    stdout.write_all(&report)?;
    stdout.write_all(b"\n")?;
    Ok(())
}
