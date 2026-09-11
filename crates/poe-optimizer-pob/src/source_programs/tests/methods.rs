//! Exact complete-body lowering is separate from the full source-runtime oracle.
use super::*;
use std::path::PathBuf;

const TOOLS: &str = "src/Modules/ModTools.lua";
const STORE: &str = "src/Classes/ModStore.lua";
const LIST: &str = "src/Classes/ModList.lua";
const DB: &str = "src/Classes/ModDB.lua";

fn original_span(sources: &BTreeMap<String, String>, path: &str, name: &str) -> ItemSourceSpan {
    let text = &sources[path];
    let start = text.find(&format!("function {name}(")).unwrap();
    let end = start + text[start..].find("\nend").unwrap() + 4;
    let line = text[..start].bytes().filter(|b| *b == b'\n').count() as u32 + 1;
    let end_line = line + text[start..end].bytes().filter(|b| *b == b'\n').count() as u32;
    ItemSourceSpan {
        path: path.into(),
        line,
        end_line,
        sha256: hash(
            text.split_inclusive('\n')
                .skip(line as usize - 1)
                .take((end_line - line + 1) as usize)
                .collect::<String>()
                .as_bytes(),
        ),
    }
}

fn complete_original_methods() -> (BTreeMap<String, String>, SourceProgramOwner) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
    let sources: BTreeMap<String, String> = [TOOLS, STORE, LIST, DB]
        .into_iter()
        .map(|path| {
            (
                path.into(),
                crate::source::read_verified_text(&root, path).unwrap(),
            )
        })
        .collect();
    // IDs and named captures are test fixture anchors, not production binding proof.
    // The full-runtime oracle separately observes original constructed closures.
    let methods = [
        (
            TOOLS,
            "modLib.createMod",
            vec![("select", 12), ("type", 11)],
        ),
        (STORE, "ModStoreClass:NewMod", vec![("mod_createMod", 1)]),
        (
            STORE,
            "ModStoreClass:ReplaceMod",
            vec![("mod_createMod", 1)],
        ),
        (LIST, "ModListClass:AddMod", vec![("t_insert", 13)]),
        (DB, "ModDBClass:AddMod", vec![("t_insert", 13)]),
        (
            LIST,
            "ModListClass:ReplaceModInternal",
            vec![("ipairs", 14)],
        ),
        (DB, "ModDBClass:ReplaceModInternal", vec![]),
        (STORE, "ModStoreClass:ModStore", vec![]),
        (LIST, "ModListClass:ModList", vec![]),
        (DB, "ModDBClass:ModDB", vec![]),
    ];
    let construction_spans = methods
        .iter()
        .map(|(path, name, _)| (name.to_string(), original_span(&sources, path, name)))
        .collect();
    let mut callbacks = methods
        .into_iter()
        .map(|(path, name, captures)| ParserCallback {
            kind: ParserCallbackKind::Lua {
                source: original_span(&sources, path, name),
            },
            upvalues: captures
                .into_iter()
                .map(|(name, id)| ParserUpvalue {
                    name: name.into(),
                    value: ParserValue::Callback(ParserCallbackId(id)),
                })
                .collect(),
            environment: ParserEnvironment::OriginalGlobals,
        })
        .collect::<Vec<_>>();
    let operations = [
        ParserProgramIntrinsic::Type,
        ParserProgramIntrinsic::Select,
        ParserProgramIntrinsic::TableInsert,
        ParserProgramIntrinsic::Ipairs,
    ];
    for operation in operations {
        callbacks.push(ParserCallback {
            kind: ParserCallbackKind::Builtin {
                symbol: operation.global_path().unwrap().join("."),
            },
            upvalues: vec![],
            environment: ParserEnvironment::OriginalGlobals,
        });
    }
    let owner = SourceProgramOwner::new(SourceProgramDefinitions {
        schema_version: SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION,
        source: ItemLoadingSource {
            upstream_revision: crate::source::UPSTREAM_REVISION.into(),
            files: sources
                .iter()
                .map(|(path, text)| (path.clone(), hash(text.as_bytes())))
                .collect(),
            construction_spans,
            module_order: vec![TOOLS.into(), STORE.into(), LIST.into(), DB.into()],
        },
        tables: vec![],
        callbacks,
        roots: vec![],
        intrinsics: operations
            .into_iter()
            .enumerate()
            .map(|(index, operation)| (ParserCallbackId(index as u32 + 11), operation))
            .collect(),
    })
    .unwrap();
    (sources, owner)
}

#[test]
fn complete_original_mod_construction_store_methods_and_parent_branches_lower() {
    let (sources, owner) = complete_original_methods();
    let lowered = lower_from_sources(&sources, &owner).unwrap();
    assert!(
        lowered.unsupported().is_empty(),
        "{:?}",
        lowered.unsupported()
    );
    let programs = &lowered.catalog().data().programs;
    assert_eq!(programs.len(), 10);
    for program in programs {
        let ParserCallbackKind::Lua { source } =
            &owner.callbacks()[program.callback.0 as usize - 1].kind
        else {
            panic!("source")
        };
        assert_eq!(&program.provenance.source, source);
        assert!(program.provenance.function_end > program.provenance.function_start);
        assert!(
            !program.bindings.iter().any(|binding| matches!(
                binding,
                ParserProgramBinding::Intrinsic {
                    operation: ParserProgramIntrinsic::CreateMod,
                    ..
                }
            )),
            "constructor body replaced with game-specific intrinsic"
        );
    }
    for (index, method) in [
        (1, "AddMod"),
        (2, "ReplaceModInternal"),
        (5, "ReplaceModInternal"),
        (6, "ReplaceModInternal"),
        (8, "ModStore"),
        (9, "ModStore"),
    ] {
        assert!(programs[index].bindings.iter().any(|binding| matches!(binding, ParserProgramBinding::DynamicMethod {key} if key == method)), "complete dynamic dependency {method}");
    }
    assert_eq!(programs[0].parameter_count, 3);
    assert!(programs[0].variadic);
    assert_eq!(programs[1].parameter_count, 1);
    assert!(programs[1].variadic);
}

#[test]
fn missing_actual_constructor_primitive_removes_its_complete_static_call_closure() {
    let (sources, owner) = complete_original_methods();
    let mut definitions = owner.definitions().unwrap().clone();
    definitions.intrinsics.remove(&ParserCallbackId(12));
    let owner = SourceProgramOwner::new(definitions).unwrap();
    let lowered = lower_from_sources(&sources, &owner).unwrap();
    assert!(lowered.unsupported().contains_key(&ParserCallbackId(12)));
    for id in [1, 2, 3] {
        assert!(
            lowered.unsupported().contains_key(&ParserCallbackId(id)),
            "incomplete constructor dependency {id}"
        );
        assert!(
            !lowered
                .catalog()
                .data()
                .callbacks
                .contains_key(&ParserCallbackId(id))
        );
    }
    // Independent complete mutation bodies remain inspectable; executing their
    // unknown dynamic dependency is a runtime frontier, never body truncation.
    assert_eq!(lowered.catalog().data().programs.len(), 7);
}
