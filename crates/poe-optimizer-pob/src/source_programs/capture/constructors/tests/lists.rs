use super::*;

fn allocations(text: &str) -> crate::source_programs::SourceProgramExtraction {
    let lua = Lua::new();
    let observer = observer(&lua);
    let (sources, observed) = observe(&lua, &observer, text);
    let lowered = lower_observed_from_sources(
        &sources,
        observed.owner(),
        observed.constructor_observations().unwrap(),
    )
    .unwrap();
    assert!(
        lowered.unsupported().is_empty(),
        "{text}: {:?}",
        lowered.unsupported()
    );
    lowered
}

#[test]
fn complete_multiple_list_inventory_preserves_exact_allocation_order_and_capacities() {
    let text = "return function(x,y,f,...) local a,b={},{} return a,b,{x},{x,y},{...},{x,f()},{{x},{y}} end";
    let lowered = allocations(text);
    assert!(
        lowered.constructor_unsupported().is_empty(),
        "{:?}",
        lowered.constructor_unsupported()
    );
    let sites = &lowered.catalog().constructors().unwrap().sites;
    let body = &text[text.find("function").unwrap()..];
    let expected = [
        ("{}", 0),
        ("{}", 0),
        ("{x}", 3),
        ("{x,y}", 3),
        ("{...}", 3),
        ("{x,f()}", 3),
        ("{{x},{y}}", 3),
        ("{x}", 3),
        ("{y}", 3),
    ];
    assert_eq!(sites.len(), expected.len());
    for (site, (slice, capacity)) in sites.iter().zip(expected) {
        assert_eq!(
            &body[site.expression.start as usize..site.expression.end as usize],
            slice
        );
        assert_eq!(
            site.allocation,
            SourceTableAllocation::New {
                array_slots: capacity,
                hash_bits: 0
            }
        );
        assert_eq!(
            SourceTableAllocation::from_tnew_instruction(site.instruction).unwrap(),
            site.allocation
        );
    }
    assert!(
        sites
            .windows(2)
            .all(|pair| pair[0].bytecode_pc < pair[1].bytecode_pc)
    );
}

#[test]
fn exact_previous_token_lines_include_multiline_expressions_comments_and_parentheses() {
    for newline in ["\n", "\r\n"] {
        for statements in [
            "return\n {x}",
            "local t =\n {x}\n return t",
            "return (\n {x}\n )",
            "return {\n {x},\n -- ignored { } function\n {x}\n }",
            "local a={}\n return a,\n {x}",
            "local label = [=[literal\n { ignored }\n]=]\n return {label,x}",
        ] {
            let text = format!("return function(x)\n {statements}\nend").replace('\n', newline);
            let lowered = allocations(&text);
            assert!(
                lowered.constructor_unsupported().is_empty(),
                "{text}: {:?}",
                lowered.constructor_unsupported()
            );
            assert!(!lowered.catalog().constructors().unwrap().sites.is_empty());
        }
    }
}

#[test]
fn unsupported_template_sites_do_not_shift_other_exact_occurrences() {
    let text = "return function(x) local a={named=x} local b={} local c={0,x} return a,b,c,{x} end";
    let lowered = allocations(text);
    assert_eq!(lowered.constructor_unsupported().len(), 1);
    assert!(
        lowered
            .constructor_unsupported()
            .values()
            .next()
            .unwrap()
            .starts_with("2 unsupported constructor site(s)")
    );
    let sites = &lowered.catalog().constructors().unwrap().sites;
    assert_eq!(sites.len(), 2);
    let body = &text[text.find("function").unwrap()..];
    assert_eq!(
        &body[sites[0].expression.start as usize..sites[0].expression.end as usize],
        "{}"
    );
    assert_eq!(
        &body[sites[1].expression.start as usize..sites[1].expression.end as usize],
        "{x}"
    );
    assert!(sites[0].bytecode_pc < sites[1].bytecode_pc);
}

#[test]
fn nested_actual_prototypes_have_separate_complete_table_inventories() {
    use crate::source_programs::lower_observed_closures_and_constructors_from_sources;
    let text = "return function(x) local a=({x}) local f=(function(y) local b={} return {y},b end) return a,{f},function() return {} end end";
    let lua = Lua::new();
    let observer = SourceClosureObserver::capture_before_source_with_closures(
        &lua,
        SourceTableRuntimeProfile::luajit21_x64_single(),
    )
    .unwrap();
    let (sources, observed) = observe(&lua, &observer, text);
    let lowered = lower_observed_closures_and_constructors_from_sources(
        &sources,
        observed.owner(),
        observed.closure_observations().unwrap(),
        observed.constructor_observations().unwrap(),
    )
    .unwrap();
    assert!(
        lowered.unsupported().is_empty(),
        "{:?}",
        lowered.unsupported()
    );
    assert!(
        lowered.constructor_unsupported().is_empty(),
        "{:?}",
        lowered.constructor_unsupported()
    );
    assert_eq!(lowered.catalog().data().programs.len(), 3);
    assert_eq!(
        lowered.catalog().closure_creations().unwrap().sites.len(),
        2
    );
    let sites = &lowered.catalog().constructors().unwrap().sites;
    assert_eq!(sites.len(), 5);
    let root = observed.callbacks()["root"];
    assert_eq!(sites.iter().filter(|site| site.callback == root).count(), 2);
    for site in sites {
        let program = lowered
            .catalog()
            .data()
            .programs
            .iter()
            .find(|program| program.callback == site.callback)
            .unwrap();
        let span = &program.provenance.source;
        let source =
            crate::source_programs::capture::closures::source_slice(text, span.line, span.end_line)
                .unwrap();
        let function = &source
            [program.provenance.function_start as usize..program.provenance.function_end as usize];
        let literal = &function[site.expression.start as usize..site.expression.end as usize];
        assert!(
            literal.starts_with('{') && literal.ends_with('}'),
            "{literal}"
        );
        assert!(!literal.contains("function"));
    }
}

#[test]
fn physical_large_array_sentinel_is_observed_from_complete_list_source() {
    for fields in [1usize, 2, 3, 2045, 2046, 2047, 2048] {
        let text = format!(
            "return function(x) return {{{}}} end",
            vec!["x"; fields].join(",")
        );
        let lowered = allocations(&text);
        assert!(
            lowered.constructor_unsupported().is_empty(),
            "{fields}: {:?}",
            lowered.constructor_unsupported()
        );
        let sites = &lowered.catalog().constructors().unwrap().sites;
        assert_eq!(sites.len(), 1);
        let hint = (fields + 1).clamp(3, 2047) as u32;
        assert_eq!(sites[0].instruction >> 16, hint);
        assert_eq!(
            sites[0].allocation,
            SourceTableAllocation::New {
                array_slots: if hint == 2047 { 2049 } else { hint },
                hash_bits: 0
            }
        );
    }
}

#[test]
fn missing_occurrence_or_changed_instruction_line_rejects_evidence_without_repair() {
    let lua = Lua::new();
    let observer = observer(&lua);
    let (sources, observed) = observe(
        &lua,
        &observer,
        "return function(x) local a={} return a,{x} end",
    );
    let id = observed.callbacks()["root"];
    let mut evidence = observed.constructor_observations().unwrap().clone();
    evidence
        .callbacks
        .get_mut(&id)
        .unwrap()
        .constructors
        .remove(0);
    let lowered = lower_observed_from_sources(&sources, observed.owner(), &evidence).unwrap();
    assert_eq!(lowered.constructor_unsupported().len(), 1);
    assert!(lowered.catalog().constructors().unwrap().sites.is_empty());
    let mut evidence = observed.constructor_observations().unwrap().clone();
    evidence.callbacks.get_mut(&id).unwrap().constructors[1].line += 1;
    let lowered = lower_observed_from_sources(&sources, observed.owner(), &evidence).unwrap();
    assert_eq!(lowered.constructor_unsupported().len(), 1);
    assert_eq!(lowered.catalog().constructors().unwrap().sites.len(), 1);
}

#[test]
fn function_literals_in_list_fields_do_not_leak_child_table_occurrences() {
    let text = "return function(x) return {function(y) return {y},{} end,x},{} end";
    let lua = Lua::new();
    let observer = SourceClosureObserver::capture_before_source_with_closures(
        &lua,
        SourceTableRuntimeProfile::luajit21_x64_single(),
    )
    .unwrap();
    let (sources, observed) = observe(&lua, &observer, text);
    let lowered = crate::source_programs::lower_observed_closures_and_constructors_from_sources(
        &sources,
        observed.owner(),
        observed.closure_observations().unwrap(),
        observed.constructor_observations().unwrap(),
    )
    .unwrap();
    assert!(
        lowered.unsupported().is_empty(),
        "{:?}",
        lowered.unsupported()
    );
    assert!(
        lowered.constructor_unsupported().is_empty(),
        "{:?}",
        lowered.constructor_unsupported()
    );
    let sites = &lowered.catalog().constructors().unwrap().sites;
    assert_eq!(sites.len(), 4);
    let root = observed.callbacks()["root"];
    assert_eq!(sites.iter().filter(|site| site.callback == root).count(), 2);
    assert_eq!(
        lowered.catalog().closure_creations().unwrap().sites.len(),
        1
    );
}
