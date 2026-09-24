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
