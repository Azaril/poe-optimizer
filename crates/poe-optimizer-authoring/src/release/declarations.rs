//! Connect declaration sources and explicit allocation history to the existing
//! immutable declaration compiler. No files are read or published here.
use super::{
    Decoder, ReleaseAllocationBase, ReleaseBuildError, ReleaseBuildLimits, ReleaseManifest, Result,
};
use crate::{
    AllocationLimits, DeclarationError, NamedIdRegistry, NamesLimits, StagedStatDeclarations,
    StatDeclarations, SymbolRegistry, load_names, parse_stat_declarations, stage_stat_declarations,
};
use poe_optimizer_core::owned_content::OwnedContentDigest;
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::owned_mapping::{OwnedIdRegistry, OwnedIdRegistrySnapshot};

pub(super) fn stage(
    manifest: &ReleaseManifest,
    decoder: &Decoder<'_>,
    registry: OwnedIdRegistrySnapshot,
    schema: &OwnedDefinitionSchemaPackage,
    allocation: ReleaseAllocationBase<'_>,
    limits: ReleaseBuildLimits,
) -> Result<(StagedStatDeclarations, Option<OwnedContentDigest>)> {
    let bootstrap;
    let (names, expected, predecessor) = match allocation {
        ReleaseAllocationBase::Bootstrap => {
            registry
                .validate_limits(limits.declarations.registry)
                .map_err(DeclarationError::from)?;
            bootstrap = NamedIdRegistry::empty(
                OwnedIdRegistry::from_snapshot(registry),
                AllocationLimits {
                    symbols: limits.declarations.symbols,
                    max_requests: limits.declarations.max_declarations,
                    max_snapshot_bytes: limits.declarations.max_names_bytes,
                },
            )
            .map_err(DeclarationError::from)?;
            let expected = bootstrap.identity().map_err(DeclarationError::from)?;
            (&bootstrap, expected, None)
        }
        ReleaseAllocationBase::Predecessor {
            names,
            expected_names,
        } => {
            names
                .registry()
                .validate_limits(limits.declarations.registry)
                .map_err(DeclarationError::from)?;
            registry
                .validate_successor(names.registry())
                .map_err(DeclarationError::from)?;
            (names, expected_names, Some(expected_names))
        }
    };
    let (declarations, symbols) = if let Some(spec) = manifest.declarations {
        let declarations_path = &manifest.sources[spec.declarations].path;
        let names_path = &manifest.sources[spec.names].path;
        let declarations =
            parse_stat_declarations(decoder.text(spec.declarations)?, limits.manifest).map_err(
                |source| ReleaseBuildError::SourceManifest {
                    path: declarations_path.clone(),
                    source,
                },
            )?;
        let symbols = load_names(
            decoder.text(spec.names)?,
            schema,
            NamesLimits {
                manifest: limits.manifest,
                symbols: limits.declarations.symbols,
            },
        )
        .map_err(|source| ReleaseBuildError::SourceManifest {
            path: names_path.clone(),
            source,
        })?;
        (declarations, symbols)
    } else {
        // No new declarations still carries all predecessor allocation history
        // into the final runtime registry and successor name ledger.
        (
            StatDeclarations {
                namespace: schema.input().namespace.clone(),
                stats: vec![],
            },
            SymbolRegistry::new(schema, &[], &[], limits.declarations.symbols)
                .map_err(DeclarationError::from)?,
        )
    };
    let staged = stage_stat_declarations(
        names,
        expected,
        schema,
        &symbols,
        &declarations,
        limits.declarations,
    )
    .map_err(|source| match manifest.declarations {
        Some(spec) => ReleaseBuildError::DeclarationSource {
            path: manifest.sources[spec.declarations].path.clone(),
            names: manifest.sources[spec.names].path.clone(),
            source,
        },
        None => ReleaseBuildError::Declaration(source),
    })?;
    Ok((staged, predecessor))
}
