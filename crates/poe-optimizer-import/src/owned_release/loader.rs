//! Checked artifact consumption. No publication assembly or artifact buffers.
use super::*;
use crate::{
    owned_recipe::{OWNED_RECIPE_VERSION, RecipeArtifactManifest},
    owned_release_evaluation::OwnedReleaseSupportInput,
};
use serde::de::DeserializeOwned;

const REQUIRED: [&str; 11] = [
    "registry.json",
    "schema.json",
    "rules.json",
    "routing.json",
    "manifest.json",
    "mapping.json",
    "roles.json",
    "normalization.json",
    "rewards.json",
    "items.json",
    "item-source.json",
];

fn query_name(name: &str) -> Option<&str> {
    let label = name.strip_prefix("queries-")?.strip_suffix(".json")?;
    (!label.is_empty()
        && label.len() <= 64
        && label
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_'))
    .then_some(label)
}

fn decode<T: DeserializeOwned>(files: &BTreeMap<String, Vec<u8>>, name: &str) -> Result<T> {
    Ok(serde_json::from_slice(files.get(name).ok_or(
        OwnedReleaseError::Invalid("missing release artifact"),
    )?)?)
}

impl OwnedReleaseReceipt {
    /// Check the declared inventory before a host opens any listed path. Returns
    /// the claimed artifact bytes (excluding this receipt), not verified content.
    /// Both filesystem and in-memory readers use this same closed inventory law.
    pub fn validate_artifact_inventory(&self, limits: OwnedReleaseLimits) -> Result<usize> {
        limits.validate()?;
        let receipt = self;
        checked::input_domain(receipt.schema_version, receipt.evaluation.is_some())?;
        if receipt.document_kind != "owned_data_release"
            || receipt.query_sets > limits.max_query_sets
            || receipt.query_rows > limits.max_queries
            || receipt.provenance.len() > limits.max_provenance_entries
        {
            return Err(OwnedReleaseError::Invalid(
                "unsupported or oversized release receipt",
            ));
        }
        let mut allowed: BTreeSet<&str> = REQUIRED.into_iter().collect();
        if receipt.tree.is_some() {
            allowed.insert("tree-normalization.json");
        }
        if let Some(evaluation) = &receipt.evaluation {
            allowed.insert("metrics.json");
            if let Some(support) = &evaluation.support {
                allowed.extend([
                    "support-stages.json",
                    "support-preparation.json",
                    "support-inputs.json",
                    "support-receiving.json",
                ]);
                if support.outputs.is_some() {
                    allowed.insert("support-outputs.json");
                }
            }
        }
        let mut total = 0usize;
        if receipt.artifacts.len() > allowed.len() + limits.max_query_sets {
            return Err(OwnedReleaseError::Limit("artifact count"));
        }
        let mut names = BTreeSet::new();
        for row in &receipt.artifacts {
            if (!allowed.contains(row.file.as_str()) && query_name(&row.file).is_none())
                || !names.insert(row.file.as_str())
                || row.bytes > limits.max_artifact_bytes
                || row.sha256.len() != 64
                || !row
                    .sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(OwnedReleaseError::Invalid(
                    "invalid release artifact inventory",
                ));
            }
            total = total
                .checked_add(row.bytes)
                .ok_or(OwnedReleaseError::Limit("artifact byte overflow"))?;
        }
        if allowed.iter().any(|name| !names.contains(name))
            || names
                .iter()
                .filter(|name| query_name(name).is_some())
                .count()
                != receipt.query_sets
        {
            return Err(OwnedReleaseError::Invalid(
                "incomplete release artifact inventory",
            ));
        }
        Ok(total)
    }
}

/// Validate a complete publication using already bounded host reads. The host
/// separately rejects non-files, symlinks and unexpected directory entries.
/// This function rechecks the entire byte inventory and all semantic claims;
/// hashes alone never confer authority. Original buffers are not retained.
pub fn load_owned_release_artifacts(
    files: &BTreeMap<String, Vec<u8>>,
    limits: OwnedReleaseLimits,
) -> Result<CheckedOwnedRelease> {
    limits.validate()?;
    let maximum_files = REQUIRED
        .len()
        .saturating_add(limits.max_query_sets)
        .saturating_add(8);
    if files.len() > maximum_files {
        return Err(OwnedReleaseError::Limit("artifact count"));
    }
    let mut input_left = limits.max_input_bytes;
    let mut output_left = limits.max_output_bytes;
    for bytes in files.values() {
        if bytes.len() > limits.max_artifact_bytes {
            return Err(OwnedReleaseError::Limit("artifact bytes"));
        }
        charge(&mut input_left, bytes.len(), "input bytes")?;
        charge(&mut output_left, bytes.len(), "output bytes")?;
    }
    let receipt: OwnedReleaseReceipt = decode(files, "release.json")?;
    receipt.validate_artifact_inventory(limits)?;
    if receipt.artifacts.len() + 1 != files.len() {
        return Err(OwnedReleaseError::Invalid(
            "incomplete release artifact inventory",
        ));
    }
    for row in &receipt.artifacts {
        let bytes = files
            .get(&row.file)
            .ok_or(OwnedReleaseError::Invalid("missing release artifact"))?;
        if row.bytes != bytes.len() || row.sha256 != format!("{:x}", Sha256::digest(bytes)) {
            return Err(OwnedReleaseError::Invalid(
                "release artifact hash/size differs",
            ));
        }
    }
    let recipe = OwnedRecipeInput {
        schema_version: OWNED_RECIPE_VERSION,
        registry: decode(files, "registry.json")?,
        schema: decode(files, "schema.json")?,
        rules: decode(files, "rules.json")?,
        routing: decode(files, "routing.json")?,
    };
    let query_sets = receipt
        .artifacts
        .iter()
        .filter_map(|row| {
            query_name(&row.file).map(|name| {
                Ok(NamedQuerySet {
                    name: OwnedDefinitionKey::new(name)
                        .map_err(|_| OwnedReleaseError::QuerySetName)?,
                    queries: decode(files, &row.file)?,
                })
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let tree = receipt
        .tree
        .map(|_| decode(files, "tree-normalization.json"))
        .transpose()?;
    let evaluation = receipt
        .evaluation
        .as_ref()
        .map(|evaluation| {
            Ok::<_, OwnedReleaseError>(OwnedReleaseEvaluationInput {
                metrics: decode(files, "metrics.json")?,
                support: evaluation
                    .support
                    .as_ref()
                    .map(|support| {
                        Ok::<_, OwnedReleaseError>(OwnedReleaseSupportInput {
                            stages: decode(files, "support-stages.json")?,
                            preparation: decode(files, "support-preparation.json")?,
                            inputs: decode(files, "support-inputs.json")?,
                            receiving: decode(files, "support-receiving.json")?,
                            outputs: support
                                .outputs
                                .map(|_| decode(files, "support-outputs.json"))
                                .transpose()?,
                        })
                    })
                    .transpose()?,
            })
        })
        .transpose()?;
    let mut checked = CheckedOwnedRelease::check(
        ReleaseView {
            schema_version: receipt.schema_version,
            recipe: (&recipe).into(),
            mapping: &decode(files, "mapping.json")?,
            roles: &decode(files, "roles.json")?,
            normalization: &decode(files, "normalization.json")?,
            rewards: &decode(files, "rewards.json")?,
            items: &decode(files, "items.json")?,
            item_source: &decode(files, "item-source.json")?,
            tree: tree.as_ref(),
            evaluation: evaluation.as_ref(),
            query_sets: &query_sets,
            provenance: &receipt.provenance,
        },
        limits,
    )?;

    // Compare directly against the supplied byte slices. No encoded duplicate
    // is allocated, and constructor sorting cannot hide noncanonical wire data.
    let mut reader = CanonicalReader {
        files,
        rows: vec![],
    };
    let recipe = checked.recipe();
    reader.check("registry.json", recipe.registry().input())?;
    reader.check("schema.json", recipe.schema().input())?;
    reader.check("rules.json", recipe.rules().input())?;
    reader.check("routing.json", recipe.routing().input())?;
    let manifest = recipe.manifest(
        reader
            .rows
            .iter()
            .zip(REQUIRED)
            .map(|(row, file)| RecipeArtifactManifest {
                file,
                bytes: row.bytes,
                sha256: row.sha256.clone(),
            })
            .collect(),
    )?;
    reader.check("manifest.json", &manifest)?;
    let mut recipe_left = limits.recipe.max_output_bytes;
    for row in &reader.rows {
        charge(&mut recipe_left, row.bytes, "recipe output bytes")?;
    }
    reader.check("mapping.json", checked.mapping().input())?;
    reader.check("roles.json", checked.roles().input())?;
    reader.check("normalization.json", checked.normalization())?;
    reader.check("rewards.json", checked.rewards().input())?;
    reader.check("items.json", checked.items().input())?;
    reader.check("item-source.json", checked.item_source().input())?;
    for set in checked.query_sets() {
        reader.check(&format!("queries-{}.json", set.name.as_str()), &set.queries)?;
    }
    if let Some(tree) = checked.tree() {
        reader.check("tree-normalization.json", tree.input())?;
    }
    if let Some(evaluation) = checked.evaluation() {
        reader.check("metrics.json", evaluation.metrics().input())?;
        if let Some(support) = evaluation.support() {
            reader.check("support-stages.json", support.stages().input())?;
            reader.check("support-preparation.json", support.preparation().input())?;
            reader.check("support-inputs.json", support.inputs().input())?;
            reader.check("support-receiving.json", support.receiving().input())?;
            if let Some(outputs) = support.outputs() {
                reader.check("support-outputs.json", outputs.input())?;
            }
        }
    }
    checked.receipt.artifacts = std::mem::take(&mut reader.rows);
    // Ordered metadata, input digest, every identity, provenance, source union,
    // counts, and omitted fields all have one canonical representation.
    reader.check("release.json", checked.receipt())?;
    Ok(checked)
}

struct CanonicalReader<'a> {
    files: &'a BTreeMap<String, Vec<u8>>,
    rows: Vec<OwnedReleaseArtifactManifest>,
}
impl CanonicalReader<'_> {
    fn check<T: Serialize>(&mut self, name: &str, value: &T) -> Result<()> {
        let bytes = self
            .files
            .get(name)
            .ok_or(OwnedReleaseError::Invalid("missing release artifact"))?;
        let mut writer = CompareWriter {
            remaining: bytes,
            mismatch: false,
        };
        let result = serde_json::to_writer(&mut writer, value);
        if writer.mismatch || !writer.remaining.is_empty() {
            return Err(OwnedReleaseError::Invalid(
                "release artifact is not canonical or its claims differ",
            ));
        }
        result?;
        self.rows.push(OwnedReleaseArtifactManifest {
            file: name.into(),
            bytes: bytes.len(),
            sha256: format!("{:x}", Sha256::digest(bytes)),
        });
        Ok(())
    }
}
struct CompareWriter<'a> {
    remaining: &'a [u8],
    mismatch: bool,
}
impl io::Write for CompareWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if !self.remaining.starts_with(bytes) {
            self.mismatch = true;
            return Err(io::Error::other("canonical release bytes differ"));
        }
        self.remaining = &self.remaining[bytes.len()..];
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
