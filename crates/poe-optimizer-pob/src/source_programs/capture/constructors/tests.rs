use super::*;
use crate::source_programs::{lower_from_sources, lower_observed_from_sources};
const PATH: &str = "tests/constructor.lua";
fn source(text: &str) -> (BTreeMap<String, String>, ItemLoadingSource) {
    (
        [(PATH.into(), text.into())].into(),
        ItemLoadingSource {
            upstream_revision: "a".repeat(40),
            files: [(PATH.into(), hash(text.as_bytes()))].into(),
            construction_spans: BTreeMap::new(),
            module_order: vec![PATH.into()],
        },
    )
}
fn observer(lua: &Lua) -> SourceClosureObserver {
    SourceClosureObserver::capture_before_source_with_constructors(
        lua,
        SourceTableRuntimeProfile::luajit21_x64_single(),
    )
    .unwrap()
}
fn observe(
    lua: &Lua,
    observer: &SourceClosureObserver,
    text: &str,
) -> (BTreeMap<String, String>, ObservedSourceContext) {
    let function: Function = lua.load(text).set_name(format!("@{PATH}")).eval().unwrap();
    let (sources, source) = source(text);
    let result = observer
        .observe_with_context(
            lua,
            &sources,
            source,
            &[("root".into(), function)].into(),
            SourceCaptureContext::default(),
        )
        .unwrap();
    (sources, result)
}
#[test]
fn actual_instruction_and_exact_source_range_are_bound_without_exposing_reflection() {
    for newline in ["\n", "\r\n"] {
        let lua = Lua::new();
        let loaded: Table = lua.named_registry_value("_LOADED").unwrap();
        assert!(matches!(
            loaded.raw_get::<Value>("jit.util").unwrap(),
            Value::Nil
        ));
        let observer = observer(&lua);
        assert!(matches!(
            loaded.raw_get::<Value>("jit.util").unwrap(),
            Value::Nil
        ));
        assert!(matches!(
            lua.globals().raw_get::<Value>("debug").unwrap(),
            Value::Nil
        ));
        let text = [
            "return function()",
            "  local out = { -- retained source comment",
            "  }",
            "  return out",
            "end",
        ]
        .join(newline);
        let (sources, observed) = observe(&lua, &observer, &text);
        let ordinary = lower_from_sources(&sources, observed.owner()).unwrap();
        assert!(ordinary.catalog().constructors().is_none());
        let lowered = lower_observed_from_sources(
            &sources,
            observed.owner(),
            observed.constructor_observations().unwrap(),
        )
        .unwrap();
        assert!(lowered.unsupported().is_empty());
        assert!(
            lowered.constructor_unsupported().is_empty(),
            "{:?}",
            lowered.constructor_unsupported()
        );
        assert_eq!(ordinary.catalog().data(), lowered.catalog().data());
        let sites = &lowered.catalog().constructors().unwrap().sites;
        assert_eq!(sites.len(), 1);
        let site = &sites[0];
        assert_eq!(site.callback, observed.callbacks()["root"]);
        assert_eq!(site.instruction & 255, 52);
        assert_eq!(site.instruction >> 16, 0);
        assert!(site.bytecode_pc > 0);
        assert_eq!(site.bytecode_sha256.len(), 64);
        let function = &text[text.find("function").unwrap()..];
        assert_eq!(
            &function[site.expression.start as usize..site.expression.end as usize],
            format!("{{ -- retained source comment{newline}  }}")
        );
        assert!(matches!(
            site.allocation,
            SourceTableAllocation::New {
                array_slots: 0,
                hash_bits: 0
            }
        ));
    }
}
#[test]
fn actual_transitive_helpers_are_observed_and_zero_opt_in_keeps_legacy_behavior() {
    let text = "local function helper() return {} end\nreturn function() return helper() end\n";
    let lua = Lua::new();
    let enabled = observer(&lua);
    let (sources, observed) = observe(&lua, &enabled, text);
    let lowered = lower_observed_from_sources(
        &sources,
        observed.owner(),
        observed.constructor_observations().unwrap(),
    )
    .unwrap();
    assert_eq!(lowered.catalog().data().programs.len(), 2);
    let site = &lowered.catalog().constructors().unwrap().sites[0];
    assert_ne!(site.callback, observed.callbacks()["root"]);
    let lua = Lua::new();
    let legacy = SourceClosureObserver::capture_before_source(&lua).unwrap();
    let (sources, observed) = observe(&lua, &legacy, text);
    assert!(observed.constructor_observations().is_none());
    assert!(
        lower_from_sources(&sources, observed.owner())
            .unwrap()
            .catalog()
            .constructors()
            .is_none()
    );
}
#[test]
fn templates_multiple_occurrences_and_nonempty_tnew_are_explicit_frontiers() {
    for text in [
        "return function() local a,b={},{} return a,b end",
        "return function() return {x=1} end",
        "return function(...) return {...} end",
    ] {
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
            "{:?}",
            lowered.unsupported()
        );
        assert_eq!(lowered.constructor_unsupported().len(), 1);
        assert!(lowered.catalog().constructors().unwrap().sites.is_empty());
    }
    let lua = Lua::new();
    let observer = observer(&lua);
    let text =
        "return function(flag) local t={} if flag then error('unsupported') end return t end";
    let (sources, observed) = observe(&lua, &observer, text);
    let lowered = lower_observed_from_sources(
        &sources,
        observed.owner(),
        observed.constructor_observations().unwrap(),
    )
    .unwrap();
    assert_eq!(lowered.unsupported().len(), 1);
    assert!(lowered.catalog().data().programs.is_empty());
    assert!(lowered.catalog().constructors().unwrap().sites.is_empty());
}
#[test]
fn profile_and_owner_mismatch_reject_and_private_reflection_cannot_be_rebound() {
    let lua = Lua::new();
    let mut profile = SourceTableRuntimeProfile::luajit21_x64_single();
    profile.table_bump = true;
    assert!(SourceClosureObserver::capture_before_source_with_constructors(&lua, profile).is_err());
    let observer = observer(&lua);
    let preload: Table = lua.named_registry_value("_PRELOAD").unwrap();
    preload
        .raw_set(
            "jit.util",
            lua.load("return function() error('replacement') end")
                .eval::<Function>()
                .unwrap(),
        )
        .unwrap();
    let (sources, observed) = observe(&lua, &observer, "return function() return {} end");
    assert!(
        lower_observed_from_sources(
            &sources,
            observed.owner(),
            observed.constructor_observations().unwrap()
        )
        .is_ok()
    );
    let foreign = SourceProgramOwner::new(observed.owner().definitions().unwrap().clone()).unwrap();
    assert!(
        lower_observed_from_sources(
            &sources,
            &foreign,
            observed.constructor_observations().unwrap()
        )
        .is_err()
    );
    assert!(
        SourceClosureObserver::capture_before_source_with_constructors(
            &lua,
            SourceTableRuntimeProfile::luajit21_x64_single()
        )
        .is_err()
    );
}
#[test]
fn real_session_closures_share_prototype_evidence_without_shared_state_snapshots() {
    let lua = Lua::new();
    let observer = observer(&lua);
    let text = "local function make(n)\n return function() local t={} t[1]=n return t end\nend\nreturn {make(1),make(2)}\n";
    let values: Table = lua.load(text).set_name(format!("@{PATH}")).eval().unwrap();
    let (sources, source) = source(text);
    let observed = observer
        .observe_session(
            &lua,
            &sources,
            source,
            SourceSessionCaptureRequest {
                callbacks: [
                    ("a".into(), values.raw_get(1).unwrap()),
                    ("b".into(), values.raw_get(2).unwrap()),
                ]
                .into(),
                ..SourceSessionCaptureRequest::default()
            },
        )
        .unwrap();
    assert_eq!(observed.input().closures.len(), 2);
    assert_eq!(observed.owner().callbacks().len(), 1);
    assert_eq!(observed.input().cells.len(), 2);
    assert!(observed.owner().definitions().unwrap().tables.is_empty());
    let lowered = lower_observed_from_sources(
        &sources,
        observed.owner(),
        observed.constructor_observations().unwrap(),
    )
    .unwrap();
    assert_eq!(lowered.catalog().constructors().unwrap().sites.len(), 1);
}
#[test]
fn evidence_mismatch_and_instruction_budget_fail_closed() {
    let lua = Lua::new();
    let observer = observer(&lua);
    let (sources, observed) = observe(&lua, &observer, "return function() return {} end");
    let mut evidence = observed.constructor_observations().unwrap().clone();
    let id = observed.callbacks()["root"];
    evidence.callbacks.get_mut(&id).unwrap().source.sha256 = "0".repeat(64);
    assert!(lower_observed_from_sources(&sources, observed.owner(), &evidence).is_err());
    let function: Function = lua
        .load("return function() return {} end")
        .set_name(format!("@{PATH}"))
        .eval()
        .unwrap();
    let record = &observed.constructor_observations().unwrap().callbacks[&id];
    assert!(
        observer
            .constructor_reflection
            .as_ref()
            .unwrap()
            .observe(&function, &record.source, 0)
            .is_err()
    );
    let mut evidence = observed.constructor_observations().unwrap().clone();
    evidence.callbacks.get_mut(&id).unwrap().constructors[0].word |= 1 << 16;
    let lowered = lower_observed_from_sources(&sources, observed.owner(), &evidence).unwrap();
    assert_eq!(lowered.constructor_unsupported().len(), 1);
    assert!(lowered.catalog().constructors().unwrap().sites.is_empty());
}

#[test]
fn different_actual_bytecode_cannot_merge_under_one_declared_session_prototype() {
    let lua = Lua::new();
    let observer = observer(&lua);
    let text = "return function() return {} end";
    let one: Function = lua.load(text).set_name(format!("@{PATH}")).eval().unwrap();
    let two: Function = lua
        .load("return function() return {x=1} end")
        .set_name(format!("@{PATH}"))
        .eval()
        .unwrap();
    let (sources, source) = source(text);
    let result = observer.observe_session(
        &lua,
        &sources,
        source,
        SourceSessionCaptureRequest {
            callbacks: [("one".into(), one), ("two".into(), two)].into(),
            ..SourceSessionCaptureRequest::default()
        },
    );
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("differing actual constructor bytecode")
    );
}
