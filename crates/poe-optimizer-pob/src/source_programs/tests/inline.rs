//! Callback spans come from Lua debug information, including surrounding table syntax.
use super::*;
use crate::source_programs::capture::{ObservedSourceClosures, SourceClosureObserver};

fn observe_rows(text: &str) -> ObservedSourceClosures {
    let lua = Lua::new();
    let observer = SourceClosureObserver::capture_before_source(&lua).unwrap();
    let rows: mlua::Table = lua.load(text).set_name(format!("@{PATH}")).eval().unwrap();
    let roots = rows
        .sequence_values::<mlua::Table>()
        .map(|row| {
            let row = row.unwrap();
            (
                row.raw_get::<String>("var").unwrap(),
                row.raw_get::<mlua::Function>("apply").unwrap(),
            )
        })
        .collect();
    observer
        .observe(
            &lua,
            &sources(text),
            definitions(text, vec![]).source,
            &roots,
        )
        .unwrap()
}

#[test]
fn original_inline_config_options_lower_the_entire_observed_callback() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/path-of-building-poe2");
    let original =
        crate::source::read_verified_text(&root, "src/Modules/ConfigOptions.lua").unwrap();
    // Execute these exact three original rows in a small construction fixture;
    // the integration oracle separately loads the complete original runtime.
    let start = original
        .find("\t{ var = \"multiplierCurrentManaPercentage\"")
        .unwrap();
    let end = original.find("\t{ var = \"conditionFullLife\"").unwrap();
    let text = format!(
        "local m_min = math.min\nlocal m_max = math.max\nreturn {{\n{}}}\n",
        &original[start..end]
    );
    let observed = observe_rows(&text);
    let owner = SourceProgramOwner::new(observed.definitions().clone()).unwrap();
    let lowered = lower_from_sources(&sources(&text), &owner).unwrap();
    assert!(
        lowered.unsupported().is_empty(),
        "{:?}",
        lowered.unsupported()
    );
    assert_eq!(lowered.catalog().data().programs.len(), 3);
    for program in &lowered.catalog().data().programs {
        assert_eq!(program.parameter_count, 3);
        let provenance = &program.provenance;
        let full_span = text
            .split_inclusive('\n')
            .skip(provenance.source.line as usize - 1)
            .take((provenance.source.end_line - provenance.source.line + 1) as usize)
            .collect::<String>();
        assert!(full_span.trim_end().ends_with("end },"));
        let function =
            &full_span[provenance.function_start as usize..provenance.function_end as usize];
        assert!(function.starts_with("function(val, modList, enemyModList)"));
        assert!(function.ends_with("end"));
        assert_eq!(provenance.function_sha256, hash(function.as_bytes()));
        assert_eq!(provenance.source.sha256, hash(full_span.as_bytes()));
        assert!(program.bindings.iter().any(|binding| matches!(binding,
            ParserProgramBinding::DynamicMethod { key } if key == "NewMod")));
    }
    let stationary = observed.callbacks()["conditionStationary"];
    let program = lowered
        .catalog()
        .data()
        .programs
        .iter()
        .find(|program| program.callback == stationary)
        .unwrap();
    assert_eq!(
        program.body.len(),
        4,
        "both branches and intervening statements survive"
    );
    assert!(matches!(
        program.body[0].operation,
        ParserProgramStatementKind::If { .. }
    ));
    assert!(matches!(
        program.body[3].operation,
        ParserProgramStatementKind::If { .. }
    ));
}

#[test]
fn inline_extent_ignores_literal_keywords_and_keeps_nested_supported_blocks() {
    let text = r#"return {
    { var = "nested", label = "function end", apply = function(items)
        -- function() if true then end end
        local total = 0
        local ignored = [=[function() repeat until false end]=]
        for _, value in ipairs(items) do
            if value > 0 then
                total = total + value
            elseif value == 0 then
                total = total + 2
            else
                total = total - value
            end
        end
        return total
    end },
}
"#;
    let observed = observe_rows(text);
    let owner = SourceProgramOwner::new(observed.definitions().clone()).unwrap();
    let lowered = lower_from_sources(&sources(text), &owner).unwrap();
    assert!(
        lowered.unsupported().is_empty(),
        "{:?}",
        lowered.unsupported()
    );
    let program = &lowered.catalog().data().programs[0];
    assert_eq!(program.body.len(), 4);
    assert!(matches!(
        program.body[2].operation,
        ParserProgramStatementKind::ForEach { .. }
    ));
    assert!(matches!(
        program.body[3].operation,
        ParserProgramStatementKind::Return { .. }
    ));
}

#[test]
fn inline_isolation_never_discards_unsupported_interior_branches() {
    for interior in [
        "if false then while true do break end end",
        "if false then repeat local value = 1 until value == 1 end",
        "if false then unmodeledConstructor() end",
    ] {
        let text = format!(
            "return {{{{ var = 'one', apply = function(value) local good = value + 1 {interior} return good end }}}}\n"
        );
        let observed = observe_rows(&text);
        let owner = SourceProgramOwner::new(observed.definitions().clone()).unwrap();
        let lowered = lower_from_sources(&sources(&text), &owner).unwrap();
        assert!(lowered.catalog().data().programs.is_empty(), "{interior}");
        assert_eq!(lowered.unsupported().len(), 1, "{interior}");
        assert!(
            !lowered
                .unsupported()
                .values()
                .next()
                .unwrap()
                .contains("unconsumed source")
        );
    }
}

#[test]
fn same_line_function_occurrences_remain_ambiguous() {
    let text = "return {{var='first', apply=function() return 1 end}, {var='second', apply=function() return 2 end}}\n";
    let observed = observe_rows(text);
    let owner = SourceProgramOwner::new(observed.definitions().clone()).unwrap();
    let lowered = lower_from_sources(&sources(text), &owner).unwrap();
    assert!(lowered.catalog().data().programs.is_empty());
    assert_eq!(lowered.unsupported().len(), 2);
    assert!(lowered.unsupported().values().all(|reason|
        reason.contains("source span must contain exactly one complete function")));
}

#[test]
fn inline_extent_does_not_expand_the_legacy_parser_span_policy() {
    let text = "return {{var='one', apply=function(value) return value + 1 end}}\n";
    let observed = observe_rows(text);
    let owner = SourceProgramOwner::new(observed.definitions().clone()).unwrap();
    let lowered = lower_from_sources(&sources(text), &owner).unwrap();
    assert!(lowered.unsupported().is_empty());
    let lua = Lua::new();
    let mut budget = Budget::default();
    let bindings = LoweringBindings::default();
    let legacy = Lowerer::new(
        &lua,
        text,
        ParserCallbackId(1),
        &observed.definitions().callbacks[0],
        &bindings,
        &mut budget,
    )
    .unwrap()
    .program(&span(text, 1, 1));
    assert_eq!(
        legacy.unwrap_err(),
        "unconsumed source after complete function"
    );
}

#[test]
fn inline_extent_rejects_unterminated_or_mismatched_blocks() {
    for text in [
        "{apply=function(value) if value then return value end },\n",
        "{apply=function(value) repeat value = value + 1 end end },\n",
    ] {
        let owner =
            SourceProgramOwner::new(definitions(text, vec![callback(text, 1, 1, vec![])])).unwrap();
        let lowered = lower_from_sources(&sources(text), &owner).unwrap();
        assert!(lowered.catalog().data().programs.is_empty());
        assert_eq!(lowered.unsupported().len(), 1);
        assert!(
            lowered
                .unsupported()
                .values()
                .next()
                .unwrap()
                .contains("block")
        );
    }
}
