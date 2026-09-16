//! Offline finite base-template conversion; no source checkout in the native host.
use super::owned_tree_cli::{invalid, load_checked_bundle, read};
use poe_optimizer_import::{
    owned_item_bases::{ItemBaseLimits, ItemBasePolicy, compile_owned_item_bases},
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
    /// Prior published owned bundle with its checked artifact manifest.
    input: PathBuf,
    /// Finite item-base catalog; this command never executes source code.
    #[arg(long)]
    catalog: PathBuf,
    /// Exact artifact digest, template IDs and source-header conversion policy.
    #[arg(long)]
    policy: PathBuf,
    /// Ordered ItemTemplate and EquipmentUse Capability descriptors to append.
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
    // A full published bundle includes both recipe and constituent artifacts.
    // Budget it independently from the caller's authoring inputs so a valid
    // near-limit predecessor can be converted again. Both groups retain a
    // bounded 64 MiB aggregate; constituent/compiler limits still apply.
    let mut remaining = limits.max_input_bytes;
    let catalog = read(&args.catalog, &mut remaining)?;
    let policy: ItemBasePolicy = serde_json::from_slice(&read(&args.policy, &mut remaining)?)?;
    let definitions = serde_json::from_slice(&read(&args.definitions, &mut remaining)?)?;
    let tree = TreePolicyTransitionInput::RebindPrior {
        prior: Box::new(
            prior
                .tree
                .clone()
                .ok_or_else(|| invalid("item-base compilation requires a prior tree policy"))?,
        ),
    };
    let augmented = super::owned_definition_cli::append_definitions(
        &prior.base,
        &prior.input.prior,
        definitions,
        super::owned_definition_cli::AppendKind::ItemBases,
        limits,
    )?;
    let staged = transition_owned_catalog_with_tree(
        prior.successor_input(augmented),
        CatalogAppend {
            mappings: vec![],
            source: prior.mapping.input().source.clone(),
            item_policies: CatalogItemPolicyMode::RebindPrior,
        },
        tree.clone(),
        limits,
    )?;
    prior.check_transition(&staged)?;
    let compiled = compile_owned_item_bases(
        staged.assembled(),
        staged.mapping(),
        staged.items(),
        staged.item_source(),
        &catalog,
        &policy,
        ItemBaseLimits::default(),
    )?;
    let mut input = prior.successor_input(compiled.successor);
    input.items = compiled.items;
    input.item_source = compiled.item_source;
    let finalized = transition_owned_catalog_with_tree(input, compiled.append, tree, limits)?;
    prior.check_transition(&finalized)?;
    super::owned_recipe_cli::publish_artifacts(&args.output, finalized.artifacts())?;
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(
        &mut stdout,
        &serde_json::json!({
            "item_bases": compiled.receipt, "publication": finalized.transition()
        }),
    )?;
    stdout.write_all(b"\n")?;
    Ok(())
}

#[cfg(feature = "pob")]
#[derive(clap::Args)]
pub(crate) struct ExportArgs {
    /// Optional pinned PoB checkout; only offline audited data construction is used.
    #[arg(long)]
    source_root: PathBuf,
    /// New directory receiving finite catalog and independent acquisition evidence.
    #[arg(long)]
    output: PathBuf,
}
#[cfg(feature = "pob")]
pub(crate) fn export(args: ExportArgs) -> Result<(), Box<dyn Error>> {
    let result = poe_optimizer_pob::owned_item_bases::export_owned_item_bases(
        &args.source_root,
        Default::default(),
    )?;
    let mut evidence = serde_json::to_vec_pretty(result.evidence())?;
    evidence.push(b'\n');
    super::owned_recipe_cli::publish_artifacts(
        &args.output,
        [
            ("catalog.json", result.catalog_bytes()),
            ("evidence.json", evidence.as_slice()),
        ],
    )?;
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(
        &mut stdout,
        &serde_json::json!({
            "catalog_sha256": result.evidence().catalog_sha256,
            "source_catalog_sha256": result.evidence().source_catalog_sha256,
            "bytes": result.catalog_bytes().len(),
            "bases": result.evidence().bases,
            "table_weapon_fields": result.evidence().table_weapon_fields,
            "absent_weapon_fields": result.evidence().absent_weapon_fields,
            "unsupported_weapon_fields": result.evidence().unsupported_weapon_fields,
        }),
    )?;
    stdout.write_all(b"\n")?;
    Ok(())
}
