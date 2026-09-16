//! Pure Rust offline conversion of a finite raw weapon-profile catalog.
use super::owned_tree_cli::{invalid, load_checked_bundle, read};
use poe_optimizer_import::{
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleLimits, TreePolicyTransitionInput,
        transition_owned_catalog_with_tree_compact,
    },
    owned_weapon_profiles::{
        WeaponProfileLimits, WeaponProfilePolicy, compile_owned_weapon_profiles,
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
    /// Exact base identity/source-presence catalog paired with these profiles.
    #[arg(long)]
    base_catalog: PathBuf,
    /// Reviewed catalog digests, source fields, absence behavior and typed bindings.
    #[arg(long)]
    policy: PathBuf,
    /// Ordered EquipmentUse statistics/capabilities and units to append.
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
    let mut remaining = limits.max_input_bytes;
    let catalog = read(&args.catalog, &mut remaining)?;
    let base_catalog = read(&args.base_catalog, &mut remaining)?;
    let policy: WeaponProfilePolicy = serde_json::from_slice(&read(&args.policy, &mut remaining)?)?;
    let definitions = serde_json::from_slice(&read(&args.definitions, &mut remaining)?)?;
    let tree =
        TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(prior.tree.clone().ok_or_else(|| {
                invalid("weapon profile compilation requires a prior tree policy")
            })?),
        };
    let append = CatalogAppend {
        mappings: vec![],
        source: prior.mapping.input().source.clone(),
        item_policies: CatalogItemPolicyMode::RebindPrior,
    };
    let augmented = super::owned_definition_cli::append_definitions(
        &prior.base,
        &prior.input.prior,
        definitions,
        super::owned_definition_cli::AppendKind::EquipmentNumbers,
        limits,
    )?;
    let staged = transition_owned_catalog_with_tree_compact(
        prior.successor_input(augmented),
        append.clone(),
        tree.clone(),
        limits,
    )?;
    prior.check_transition(&staged)?;
    let compiled = compile_owned_weapon_profiles(
        staged.assembled(),
        staged.mapping(),
        &base_catalog,
        &catalog,
        &policy,
        WeaponProfileLimits::default(),
    )?;
    let finalized = transition_owned_catalog_with_tree_compact(
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
        &serde_json::json!({"weapon_profiles":compiled.receipt,"publication":finalized.transition()}),
    )?;
    stdout.write_all(b"\n")?;
    Ok(())
}

#[cfg(feature = "pob")]
#[derive(clap::Args)]
pub(crate) struct ExportArgs {
    /// Optional pinned checkout used only for offline data acquisition.
    #[arg(long)]
    source_root: PathBuf,
    /// New directory receiving both finite catalogs and acquisition evidence.
    #[arg(long)]
    output: PathBuf,
}
#[cfg(feature = "pob")]
pub(crate) fn export(args: ExportArgs) -> Result<(), Box<dyn Error>> {
    let result = poe_optimizer_pob::owned_weapon_profiles::export_owned_weapon_profiles(
        &args.source_root,
        Default::default(),
    )?;
    let mut evidence = serde_json::to_vec_pretty(result.evidence())?;
    evidence.push(b'\n');
    super::owned_recipe_cli::publish_artifacts(
        &args.output,
        [
            ("catalog.json", result.catalog_bytes()),
            ("base-catalog.json", result.base_export().catalog_bytes()),
            ("evidence.json", evidence.as_slice()),
        ],
    )?;
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(
        &mut stdout,
        &serde_json::json!({
            "profiles": result.catalog().profiles.len(),
            "bytes": result.catalog_bytes().len(),
            "base_catalog_sha256": result.catalog().base_catalog_sha256,
        }),
    )?;
    stdout.write_all(b"\n")?;
    Ok(())
}
