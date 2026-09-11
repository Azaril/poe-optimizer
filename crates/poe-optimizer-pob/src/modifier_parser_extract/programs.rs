//! Bounded whole-function lowering into source-bound typed programs.
//! This pass never executes a source body or changes a legacy factory recipe.
use super::*;
use crate::source_programs::lowering::{Budget, Lowerer, LoweringBindings};

pub(super) struct LoweredPrograms {
    pub(super) data: ParserProgramData,
    pub(super) unsupported: BTreeMap<ParserCallbackId, String>,
}

pub(super) fn lower(
    lua: &Lua,
    sources: &BTreeMap<String, String>,
    data: &ModifierParserData,
    constructor: ParserCallbackId,
) -> Result<LoweredPrograms> {
    let authorization = parser_bindings(constructor);
    let mut programs = BTreeMap::new();
    let mut unsupported = BTreeMap::new();
    let mut budget = Budget::default();
    for (index, callback) in data.callbacks.iter().enumerate() {
        let id = ParserCallbackId(index as u32 + 1);
        if matches!(
            data.factories.get(&id),
            Some(ParserFactoryDisposition::Pure(_))
        ) {
            continue;
        }
        let ParserCallbackKind::Lua { source: span } = &callback.kind else {
            unsupported.insert(id, "builtin callback is not a source program".into());
            continue;
        };
        let text = source(sources, &span.path)?;
        let body = text
            .split_inclusive('\n')
            .skip(span.line as usize - 1)
            .take((span.end_line - span.line + 1) as usize)
            .collect::<String>();
        if hash(body.as_bytes()) != span.sha256 {
            return Err(error("program source span mismatch"));
        }
        let result = Lowerer::new(lua, &body, id, callback, &authorization, &mut budget)
            .and_then(|lowerer| lowerer.program(span));
        match result {
            Ok(program) => {
                programs.insert(id, program);
            }
            Err(reason) => {
                unsupported.insert(id, reason);
            }
        }
    }
    // A source call is meaningful only with its captured callee's own complete
    // body. Remove dependants transitively; never inline or invent that body.
    loop {
        let rejected = programs.iter().filter_map(|(id, program)| {
            program.bindings.iter().find_map(|binding| {
                let ParserProgramBinding::CapturedCallback { callback, .. } = binding else { return None };
                if programs.contains_key(callback) { return None; }
                let reason = if matches!(data.factories.get(callback), Some(ParserFactoryDisposition::Pure(_))) {
                    format!("captured helper {callback:?} requires the unavailable raw legacy factory bridge")
                } else {
                    format!("captured helper {callback:?} has no complete lowered program")
                };
                Some((*id, reason))
            })
        }).collect::<Vec<_>>();
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
    Ok(LoweredPrograms {
        data: ParserProgramData {
            schema_version: PARSER_PROGRAM_SCHEMA_VERSION,
            programs: programs.into_values().collect(),
            callbacks,
        },
        unsupported,
    })
}

fn parser_bindings(constructor: ParserCallbackId) -> LoweringBindings {
    LoweringBindings {
        roots: [
            (
                "ModFlag".into(),
                ParserProgramDefinitionRoot::ModFlags.into(),
            ),
            (
                "KeywordFlag".into(),
                ParserProgramDefinitionRoot::KeywordFlags.into(),
            ),
            (
                "SkillType".into(),
                ParserProgramDefinitionRoot::SkillTypes.into(),
            ),
        ]
        .into(),
        intrinsics: [(constructor, ParserProgramIntrinsic::CreateMod)].into(),
        implicit_self: false,
        standalone_calls: false,
    }
}

#[cfg(test)]
use crate::source_programs::lowering::LowerResult;
#[cfg(test)]
mod tests;
