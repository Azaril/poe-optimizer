//! Checked offline schema knowledge and normalization policy publications.
use super::owned_tree_cli::{invalid, load_checked_bundle, read};
use poe_optimizer_import::{
    owned_gem_schema::{GemSchemaMigrationInput, GemSchemaMigrationLimits, stage_owned_gem_schema},
    owned_normalize::NormalizationPolicy,
    owned_skill_catalog::OwnedSkillRoleIndex,
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleLimits, TreePolicyTransitionInput,
        transition_owned_catalog_with_gem_refinement_compact,
        transition_owned_normalization_with_tree_compact,
    },
};
use std::{
    error::Error,
    io::{self, Write},
    path::PathBuf,
};

#[derive(clap::Args)]
pub(crate) struct SchemaArgs {
    /// Checked prior owned bundle directory.
    input: PathBuf,
    /// Exact prior-bound physical Gem schemas and parameter declarations.
    #[arg(long)]
    migration: PathBuf,
    /// New destination directory; an existing directory is never replaced.
    #[arg(long)]
    output: PathBuf,
}
#[derive(clap::Args)]
pub(crate) struct NormalizationArgs {
    /// Checked prior owned bundle directory.
    input: PathBuf,
    /// Complete replacement policy bound to the exact unchanged schema.
    #[arg(long)]
    normalization: PathBuf,
    /// New destination directory; an existing directory is never replaced.
    #[arg(long)]
    output: PathBuf,
}
fn receipt(value: &serde_json::Value) -> Result<(), Box<dyn Error>> {
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, value)?;
    stdout.write_all(b"\n")?;
    Ok(())
}
pub(crate) fn schemas(args: SchemaArgs) -> Result<(), Box<dyn Error>> {
    let limits = SuccessorBundleLimits::default();
    let mut remaining = limits.max_input_bytes;
    let prior = load_checked_bundle(&args.input, &mut remaining, limits)?;
    let migration_limits = GemSchemaMigrationLimits::default();
    let mut remaining = migration_limits.max_wire_bytes;
    let migration: GemSchemaMigrationInput =
        serde_json::from_slice(&read(&args.migration, &mut remaining)?)?;
    let roles = OwnedSkillRoleIndex::new(
        prior.input.roles.clone(),
        &prior.mapping,
        prior.base.schema(),
        limits.catalog,
    )?;
    let staged = stage_owned_gem_schema(
        &prior.base,
        &prior.mapping,
        &roles,
        &migration,
        migration_limits,
    )?;
    let tree = TreePolicyTransitionInput::RebindPrior {
        prior: Box::new(
            prior
                .tree
                .clone()
                .ok_or_else(|| invalid("Gem migration requires a prior tree policy"))?,
        ),
    };
    let finalized = transition_owned_catalog_with_gem_refinement_compact(
        prior.successor_input(staged.successor),
        CatalogAppend {
            mappings: vec![],
            source: prior.mapping.input().source.clone(),
            item_policies: CatalogItemPolicyMode::RebindPrior,
        },
        tree,
        staged.refinement,
        limits,
    )?;
    prior.check_transition(&finalized)?;
    super::owned_recipe_cli::publish_artifacts(&args.output, finalized.artifacts())?;
    receipt(
        &serde_json::json!({"migration": staged.receipt, "publication": finalized.transition()}),
    )
}
pub(crate) fn normalization(args: NormalizationArgs) -> Result<(), Box<dyn Error>> {
    let limits = SuccessorBundleLimits::default();
    let mut remaining = limits.max_input_bytes;
    let prior = load_checked_bundle(&args.input, &mut remaining, limits)?;
    let mut remaining = limits.normalization.max_policy_bytes;
    let policy: NormalizationPolicy =
        serde_json::from_slice(&read(&args.normalization, &mut remaining)?)?;
    let tree = prior
        .tree
        .clone()
        .ok_or_else(|| invalid("normalization publication requires a prior tree policy"))?;
    let finalized = transition_owned_normalization_with_tree_compact(
        prior.input.clone(),
        tree,
        policy,
        limits,
    )?;
    prior.check_transition(&finalized)?;
    super::owned_recipe_cli::publish_artifacts(&args.output, finalized.artifacts())?;
    receipt(&serde_json::json!({"publication": finalized.transition()}))
}

#[derive(clap::Args)]
pub(crate) struct CatalogArgs {
    /// Checked prior owned bundle directory.
    input: PathBuf,
    /// Finite identity catalog; no PoB checkout or source execution.
    #[arg(long)]
    catalog: PathBuf,
    /// Reviewed physical-Gem schema and lexical input policy.
    #[arg(long)]
    policy: PathBuf,
    /// New directory for migration.json, normalization.json and receipt.json.
    #[arg(long)]
    output: PathBuf,
}

/// Prepare two independently checked publications. The emitted normalization
/// binds the schema migration's result, never a forged prior policy identity.
pub(crate) fn catalog(args: CatalogArgs) -> Result<(), Box<dyn Error>> {
    use poe_optimizer_data::skill_identities::SkillIdentityCatalog;
    use poe_optimizer_import::{
        owned_gem_catalog::{
            PhysicalGemCatalogLimits, PhysicalGemSchemaPolicy, compile_owned_gem_catalog,
        },
        owned_normalize::GemInputPolicy,
        owned_successor::{OWNED_SUCCESSOR_VERSION, SuccessorBundleInput},
    };
    let limits = SuccessorBundleLimits::default();
    let mut remaining = limits.max_input_bytes;
    let prior = load_checked_bundle(&args.input, &mut remaining, limits)?;
    let compilation_limits = PhysicalGemCatalogLimits::default();
    let mut remaining = compilation_limits.max_wire_bytes;
    let identities = SkillIdentityCatalog::new(serde_json::from_slice(&read(
        &args.catalog,
        &mut remaining,
    )?)?)?;
    let mut remaining = compilation_limits.max_wire_bytes;
    let policy: PhysicalGemSchemaPolicy =
        serde_json::from_slice(&read(&args.policy, &mut remaining)?)?;
    let roles = OwnedSkillRoleIndex::new(
        prior.input.roles.clone(),
        &prior.mapping,
        prior.base.schema(),
        limits.catalog,
    )?;
    let compiled = compile_owned_gem_catalog(
        &prior.base,
        &prior.mapping,
        &roles,
        &identities,
        &policy,
        compilation_limits,
    )?;
    let schema = transition_owned_catalog_with_gem_refinement_compact(
        prior.successor_input(compiled.staged.successor.clone()),
        CatalogAppend {
            mappings: vec![],
            source: prior.mapping.input().source.clone(),
            item_policies: CatalogItemPolicyMode::RebindPrior,
        },
        TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(
                prior
                    .tree
                    .clone()
                    .ok_or_else(|| invalid("Gem catalog requires a prior tree policy"))?,
            ),
        },
        compiled.staged.refinement.clone(),
        limits,
    )?;
    prior.check_transition(&schema)?;
    let mut normalization = schema.normalization().clone();
    let inputs = normalization
        .gem_inputs
        .get_or_insert_with(|| GemInputPolicy {
            definitions: schema.assembled().schema().identity().clone(),
            gems: vec![],
        });
    inputs.gems.extend(compiled.inputs);
    inputs.gems.sort_by(|a, b| a.gem.cmp(&b.gem));
    let final_input = SuccessorBundleInput {
        schema_version: OWNED_SUCCESSOR_VERSION,
        prior: schema.recipe().clone(),
        successor: schema.recipe().clone(),
        mapping: schema.mapping().input().clone(),
        roles: schema.roles().input().clone(),
        normalization: schema.normalization().clone(),
        rewards: schema.rewards().input().clone(),
        query_sets: schema.query_sets().to_vec(),
        items: schema.items().input().clone(),
        item_source: schema.item_source().input().clone(),
    };
    let final_bundle = transition_owned_normalization_with_tree_compact(
        final_input,
        schema
            .tree()
            .ok_or_else(|| invalid("Gem catalog lost tree policy"))?
            .input()
            .clone(),
        normalization,
        limits,
    )?;
    let report = serde_json::json!({
        "compilation": compiled.receipt,
        "schema_publication": schema.transition(),
        "input_publication": final_bundle.transition(),
        "calculation": "not_run", "whole_build_parity": "not_established",
    });
    let artifacts = [
        ("migration.json", serde_json::to_vec(&compiled.migration)?),
        (
            "normalization.json",
            serde_json::to_vec(final_bundle.normalization())?,
        ),
        ("receipt.json", serde_json::to_vec(&report)?),
    ];
    super::owned_recipe_cli::publish_artifacts(
        &args.output,
        artifacts
            .iter()
            .map(|(name, bytes)| (*name, bytes.as_slice())),
    )?;
    receipt(&report)
}
