//! Pure Rust offline conversion of a finite intrinsic-attack catalog.
use super::owned_tree_cli::{invalid, load_checked_bundle, read};
use poe_optimizer_import::{
    owned_intrinsic_attack::{
        IntrinsicAttackLimits, IntrinsicAttackPolicy, compile_owned_intrinsic_attack,
    },
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleLimits, TreePolicyTransitionInput,
        transition_owned_catalog_with_tree,
    },
};
use std::{
    error::Error,
    io::{self, Write},
    path::PathBuf,
};

#[derive(clap::Args)]
pub(crate) struct Args {
    /// Prior published owned bundle, including its checked artifact manifest.
    input: PathBuf,
    /// Finite source-adapter catalog. This command never executes Lua.
    #[arg(long)]
    catalog: PathBuf,
    /// Reviewed catalog digest, source field mappings and explicit exclusions.
    #[arg(long)]
    policy: PathBuf,
    /// Ordered owned Actor-stat and unit descriptors to append to the same ledger.
    #[arg(long)]
    definitions: PathBuf,
    /// New destination directory; an existing output is never replaced.
    #[arg(long)]
    output: PathBuf,
}
pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let limits = SuccessorBundleLimits::default();
    let mut remaining = limits.max_input_bytes;
    let prior = load_checked_bundle(&args.input, &mut remaining, limits)?;
    let catalog = read(&args.catalog, &mut remaining)?;
    let policy: IntrinsicAttackPolicy =
        serde_json::from_slice(&read(&args.policy, &mut remaining)?)?;
    let definitions = serde_json::from_slice(&read(&args.definitions, &mut remaining)?)?;
    let tree =
        TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(prior.tree.clone().ok_or_else(|| {
                invalid("intrinsic attack compilation requires a prior tree policy")
            })?),
        };
    let append = CatalogAppend {
        mappings: vec![],
        source: prior.mapping.input().source.clone(),
        item_policies: CatalogItemPolicyMode::RebindPrior,
    };
    let augmented = super::owned_attribute_cli::augment_definitions(
        &prior.base,
        &prior.input.prior,
        definitions,
        limits,
    )?;
    let staged = transition_owned_catalog_with_tree(
        prior.successor_input(augmented),
        append.clone(),
        tree.clone(),
        limits,
    )?;
    prior.check_transition(&staged)?;
    let compiled = compile_owned_intrinsic_attack(
        staged.assembled(),
        staged.mapping(),
        &catalog,
        &policy,
        IntrinsicAttackLimits::default(),
    )?;
    let finalized = transition_owned_catalog_with_tree(
        prior.successor_input(compiled.successor),
        append,
        tree,
        limits,
    )?;
    prior.check_transition(&finalized)?;
    super::owned_recipe_cli::publish_artifacts(&args.output, finalized.artifacts())?;
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(
        &mut stdout,
        &serde_json::json!({"intrinsic_attack":compiled.receipt,"publication":finalized.transition()}),
    )?;
    stdout.write_all(b"\n")?;
    Ok(())
}

/// Optional reference-side export only. The default/native binary has no Lua dependency.
#[cfg(feature = "pob")]
#[derive(clap::Args)]
pub(crate) struct ExportArgs {
    #[arg(long)]
    data_lua: PathBuf,
    #[arg(long)]
    misc_lua: PathBuf,
    /// Explicit reviewed source pin; the optional exporter also checks its upstream manifest.
    #[arg(long)]
    source_pin: PathBuf,
    /// New directory receiving catalog.json; never replaced.
    #[arg(long)]
    output: PathBuf,
}
#[cfg(feature = "pob")]
pub(crate) fn export(args: ExportArgs) -> Result<(), Box<dyn Error>> {
    use sha2::{Digest, Sha256};
    let mut remaining = 32 * 1024 * 1024;
    let data = read(&args.data_lua, &mut remaining)?;
    let misc = read(&args.misc_lua, &mut remaining)?;
    let source = serde_json::from_slice(&read(&args.source_pin, &mut remaining)?)?;
    let catalog = poe_optimizer_pob::owned_intrinsic_attack::export_owned_intrinsic_attack(
        &misc,
        &data,
        &source,
        Default::default(),
    )?;
    let mut bytes = serde_json::to_vec_pretty(&catalog)?;
    bytes.push(b'\n');
    let digest = format!("{:x}", Sha256::digest(&bytes));
    super::owned_recipe_cli::publish_artifacts(&args.output, [("catalog.json", bytes.as_slice())])?;
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(
        &mut stdout,
        &serde_json::json!({"catalog_sha256":digest,"bytes":bytes.len(),"classes":catalog.classes.len()}),
    )?;
    stdout.write_all(b"\n")?;
    Ok(())
}
