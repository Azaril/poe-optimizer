//! Shared whole-function source lowering for immutable constructed closure owners.
//!
//! The caller supplies a validated graph observed during source construction. This
//! layer authenticates its declared source files and complete function spans; it
//! does not reconstruct or certify captured values from source text. An extraction
//! adapter must separately prove the graph and original runtime bindings. Parser
//! extraction already does so by reconstructing and comparing its complete owner.
//! Lowering alone grants neither numerical parity nor production admission.
use crate::game_data::{GameDataExtractionError, error, hash};
use mlua::Lua;
use poe_optimizer_data::modifier_parser::*;
use poe_optimizer_data::source_program::{
    SourceProgramCatalog, SourceProgramDefinitionRoot, SourceProgramOwner,
};
use std::collections::BTreeMap;

pub mod capture;
mod closures;
mod constructors;
pub use closures::{
    lower_observed_closures_and_constructors_from_sources, lower_observed_closures_from_sources,
};
pub use constructors::{attach_reserved_string_templates, lower_observed_from_sources};
pub(crate) mod lowering;
mod syntax;
pub(crate) mod tokens;
use lowering::{Budget, Lowerer, LoweringBindings};

type Result<T> = std::result::Result<T, GameDataExtractionError>;
const MAX_SOURCE_BYTES: usize = 256 * 1024 * 1024;
const MAX_SOURCE_FILES: usize = 4096;

#[derive(Debug, Clone)]
pub struct SourceProgramExtraction {
    catalog: SourceProgramCatalog,
    unsupported: BTreeMap<ParserCallbackId, String>,
    implementation_sha256: String,
    constructor_unsupported: BTreeMap<ParserCallbackId, String>,
}
impl SourceProgramExtraction {
    /// Logical programs remain available when no exact constructor layout proof exists.
    pub fn constructor_unsupported(&self) -> &BTreeMap<ParserCallbackId, String> {
        &self.constructor_unsupported
    }
    pub fn catalog(&self) -> &SourceProgramCatalog {
        &self.catalog
    }
    /// Whole-function failures, including unsupported captured-call dependencies.
    pub fn unsupported(&self) -> &BTreeMap<ParserCallbackId, String> {
        &self.unsupported
    }
    pub fn implementation_sha256(&self) -> &str {
        &self.implementation_sha256
    }
}

/// Lower standalone source definitions using the same scanner, syntax and typed
/// IR as the modifier parser. The owner supplies explicit named definition roots
/// and captured intrinsic identities. Familiar game names and C function labels
/// cannot introduce bindings absent from that owner. Standard global primitives
/// retain the language's declared `OriginalGlobals` contract without an explicit
/// environment. With an observed environment root, every ordinary global resolves
/// through that root, including familiar primitive and named-definition names.
/// The source-construction adapter authenticates the selected contract.
///
/// Source files must exactly match the owner inventory. Unsupported functions are
/// retained as diagnostics and removed transitively from executable call closures.
/// A successful result can therefore contain no programs. Inspect `unsupported`;
/// a complete source body is never silently truncated to its supported statements.
///
/// Parser owners must use `parser_programs::extract_from_sources`, which also
/// authenticates parser construction and preserves its separate admission policy.
pub fn lower_from_sources(
    sources: &BTreeMap<String, String>,
    owner: &SourceProgramOwner,
) -> Result<SourceProgramExtraction> {
    lower_inner(sources, owner, None)
}
fn lower_inner(
    sources: &BTreeMap<String, String>,
    owner: &SourceProgramOwner,
    closures: Option<(
        &BTreeMap<ParserCallbackId, closures::BoundFunction>,
        &poe_optimizer_data::source_program::SourceTableRuntimeProfile,
    )>,
) -> Result<SourceProgramExtraction> {
    if owner.parser().is_some() {
        return Err(error(
            "parser source owners require the parser extraction adapter",
        ));
    }
    validate_sources(sources, owner)?;
    let mut bindings = LoweringBindings {
        implicit_self: true,
        standalone_calls: true,
        environment: owner.environment_root(),
        ..LoweringBindings::default()
    };
    if let Some((functions, _)) = closures {
        bindings.closure_functions = functions.clone();
        for id in functions.keys() {
            if let Some(prototype) = owner.closure_prototype_id(*id) {
                bindings.closure_prototypes.insert(*id, prototype);
            }
        }
    }
    for root in owner.roots() {
        let id = owner.root_id(&root.name).expect("validated owner root");
        bindings
            .roots
            .insert(root.name.clone(), SourceProgramDefinitionRoot::Named(id));
    }
    for (index, _) in owner.callbacks().iter().enumerate() {
        let id = ParserCallbackId(index as u32 + 1);
        if let Some(operation) = owner.intrinsic(id) {
            bindings.intrinsics.insert(id, operation);
        }
    }
    // Only bounded literals are evaluated in this fresh private VM. The caller's
    // closures are never executed, and caller-mutated globals cannot affect it.
    let lua = Lua::new();
    let mut budget = Budget::default();
    let mut programs = BTreeMap::new();
    let mut unsupported = BTreeMap::new();
    let mut creation_sites = BTreeMap::new();
    for (index, callback) in owner.callbacks().iter().enumerate() {
        let id = ParserCallbackId(index as u32 + 1);
        if bindings.intrinsics.contains_key(&id) {
            continue;
        }
        let ParserCallbackKind::Lua { source: span } = &callback.kind else {
            unsupported.insert(
                id,
                "builtin callback has no declared intrinsic binding".into(),
            );
            continue;
        };
        let text = sources
            .get(&span.path)
            .ok_or_else(|| error("missing program source file"))?;
        if span.line == 0
            || span.end_line < span.line
            || span.end_line as usize > text.lines().count()
        {
            return Err(error("program source span is outside its declared file"));
        }
        let body = text
            .split_inclusive('\n')
            .skip(span.line as usize - 1)
            .take((span.end_line - span.line + 1) as usize)
            .collect::<String>();
        if hash(body.as_bytes()) != span.sha256 {
            return Err(error("program source span mismatch"));
        }
        match Lowerer::new(&lua, &body, id, callback, &bindings, &mut budget)
            .and_then(|lowerer| lowerer.program_with_creations(span))
        {
            Ok((program, sites)) => {
                programs.insert(id, program);
                creation_sites.insert(id, sites);
            }
            Err(reason) => {
                unsupported.insert(id, reason);
            }
        }
    }
    loop {
        let rejected = programs
            .iter()
            .filter_map(|(id, program)| {
                let child_failure = creation_sites
                    .get(id)
                    .into_iter()
                    .flatten()
                    .find_map(|site| {
                        let child = owner
                            .closure_prototype(site.prototype)
                            .expect("observed child prototype")
                            .callback;
                        (!programs.contains_key(&child)).then(|| {
                            (
                                *id,
                                format!(
                                    "created child {child:?} has no complete lowered program: {}",
                                    unsupported
                                        .get(&child)
                                        .map_or("unknown body frontier", String::as_str)
                                ),
                            )
                        })
                    });
                child_failure.or_else(|| {
                    program.bindings.iter().find_map(|binding| {
                        let ParserProgramBinding::CapturedCallback { callback, .. } = binding
                        else {
                            return None;
                        };
                        (!programs.contains_key(callback)).then(|| {
                            (
                                *id,
                                format!(
                                    "captured helper {callback:?} has no complete lowered program"
                                ),
                            )
                        })
                    })
                })
            })
            .collect::<Vec<_>>();
        if rejected.is_empty() {
            break;
        }
        for (id, reason) in rejected {
            programs.remove(&id);
            unsupported.insert(id, reason);
        }
    }
    let callbacks = programs
        .keys()
        .enumerate()
        .map(|(index, callback)| (*callback, ParserProgramId(index as u32 + 1)))
        .collect();
    let data = ParserProgramData {
        schema_version: PARSER_PROGRAM_SCHEMA_VERSION,
        programs: programs.into_values().collect(),
        callbacks,
    };
    let catalog = if let Some((_, profile)) = closures {
        use poe_optimizer_data::source_program::{
            SOURCE_PROGRAM_CLOSURE_CREATIONS_SCHEMA_VERSION, SourceProgramClosureCreations,
        };
        let retained = data
            .callbacks
            .keys()
            .collect::<std::collections::BTreeSet<_>>();
        let sites = creation_sites
            .into_iter()
            .filter(|(id, _)| retained.contains(id))
            .flat_map(|(_, sites)| sites)
            .collect();
        SourceProgramCatalog::new_with_closure_creations(
            data,
            owner.clone(),
            None,
            SourceProgramClosureCreations {
                schema_version: SOURCE_PROGRAM_CLOSURE_CREATIONS_SCHEMA_VERSION,
                profile: profile.clone(),
                sites,
            },
        )
    } else {
        SourceProgramCatalog::new(data, owner.clone())
    }
    .map_err(error)?;
    Ok(SourceProgramExtraction {
        catalog,
        unsupported,
        implementation_sha256: crate::game_data::extractor_sha256(),
        constructor_unsupported: BTreeMap::new(),
    })
}

fn validate_sources(sources: &BTreeMap<String, String>, owner: &SourceProgramOwner) -> Result<()> {
    if sources.len() > MAX_SOURCE_FILES || sources.len() != owner.source().files.len() {
        return Err(error("program source file inventory mismatch or bound"));
    }
    let mut total = 0usize;
    for (path, expected) in &owner.source().files {
        let text = sources
            .get(path)
            .ok_or_else(|| error(format!("missing program source dependency: {path}")))?;
        total = total
            .checked_add(text.len())
            .ok_or_else(|| error("program source size overflow"))?;
        if total > MAX_SOURCE_BYTES {
            return Err(error("program source byte bound"));
        }
        if hash(text.as_bytes()) != *expected {
            return Err(error(format!("program owner source hash mismatch: {path}")));
        }
    }
    // Validate even opaque declared constructor functions: skipping their syntax
    // does not exempt their evidence from source authentication.
    for span in
        owner
            .source()
            .construction_spans
            .values()
            .chain(
                owner
                    .callbacks()
                    .iter()
                    .filter_map(|callback| match &callback.kind {
                        ParserCallbackKind::Lua { source } => Some(source),
                        _ => None,
                    }),
            )
    {
        let text = sources
            .get(&span.path)
            .ok_or_else(|| error("missing program span source file"))?;
        if span.line == 0
            || span.end_line < span.line
            || span.end_line as usize > text.lines().count()
        {
            return Err(error("program source span is outside its declared file"));
        }
        let body = text
            .split_inclusive('\n')
            .skip(span.line as usize - 1)
            .take((span.end_line - span.line + 1) as usize)
            .collect::<String>();
        if hash(body.as_bytes()) != span.sha256 {
            return Err(error("program source span mismatch"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
