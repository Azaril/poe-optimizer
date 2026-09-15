//! Thin host for explicit offline registry/catalog succession.
use poe_optimizer_data::skill_identities::{SkillIdentityCatalog, SkillIdentityData};
use poe_optimizer_import::{
    owned_catalog_recipe::{CatalogRecipeLimits, extend_owned_catalog_recipe},
    owned_mapping::{MappingPackageInput, SourcePin},
    owned_recipe::OwnedRecipeInput,
    owned_skill_catalog::SkillCatalogPolicy,
};
use serde::de::DeserializeOwned;
use std::{
    error::Error,
    fs::File,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

#[derive(clap::Args)]
pub(crate) struct Args {
    /// Prior owned recipe; existing IDs and descriptors must survive unchanged.
    input: PathBuf,
    /// Standalone source identity catalog JSON, used only during this data build.
    #[arg(long)]
    catalog: PathBuf,
    /// Exact prior mapping, including reviewed seeds for existing catalog identities.
    #[arg(long)]
    mapping: PathBuf,
    /// Reviewed source revision and file pins; must match the prior mapping.
    #[arg(long)]
    source: PathBuf,
    /// Explicit catalog role interpretation policy.
    #[arg(long)]
    policy: PathBuf,
    /// New output directory for the complete successor artifact family.
    #[arg(long)]
    output: PathBuf,
}
fn read<T: DeserializeOwned>(path: &Path, remaining: &mut usize) -> Result<T, Box<dyn Error>> {
    let mut bytes = vec![];
    File::open(path)?
        .take(*remaining as u64 + 1)
        .read_to_end(&mut bytes)?;
    *remaining = remaining.checked_sub(bytes.len()).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "aggregate catalog input byte limit",
        )
    })?;
    Ok(serde_json::from_slice(&bytes)?)
}
pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let limits = CatalogRecipeLimits::default();
    let mut remaining = limits.recipe.max_wire_bytes;
    let input: OwnedRecipeInput = read(&args.input, &mut remaining)?;
    let mapping: MappingPackageInput = read(&args.mapping, &mut remaining)?;
    let source: SourcePin = read(&args.source, &mut remaining)?;
    let policy: SkillCatalogPolicy = read(&args.policy, &mut remaining)?;
    let catalog =
        SkillIdentityCatalog::new(read::<SkillIdentityData>(&args.catalog, &mut remaining)?)?;
    let staged = extend_owned_catalog_recipe(input, mapping, &catalog, &source, &policy, limits)?;
    super::owned_recipe_cli::publish_artifacts(&args.output, staged.artifacts())?;
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, staged.transition())?;
    stdout.write_all(b"\n")?;
    Ok(())
}
