use super::super::{AssembledItem, AssemblyLimits, finish_no_base_jewel_radius};
use super::*;
use crate::item_loading::{
    AssemblyExecution, BuiltinItemLoadProvider, DependencyResult, ItemLoadMachine, ItemLoadStatus,
    ParseOutcome, ParseRequest,
};
use poe_optimizer_data::game_data::{GameDataSnapshot, bundled_snapshot};
use std::sync::OnceLock;

fn data() -> &'static GameDataSnapshot {
    static DATA: OnceLock<GameDataSnapshot> = OnceLock::new();
    DATA.get_or_init(|| bundled_snapshot().unwrap())
}

fn machine() -> ItemLoadMachine<'static> {
    let mut machine = ItemLoadMachine::new(data().item_loading());
    machine.set_xml_attributes(&[("id".into(), "1".into())].into());
    machine
}

fn text<'a>(item: &'a AssembledItem, field: &str) -> Option<&'a str> {
    item.field(item.root(), field).and_then(Value::as_str)
}

fn assert_base_name(item: &AssembledItem, machine: &ItemLoadMachine<'_>) {
    assert_eq!(text(item, "baseName"), machine.state().base_name.as_deref());
    assert_eq!(text(item, "type"), machine.state().item_type.as_deref());
    assert_eq!(text(item, "rarity"), Some(machine.state().rarity.as_str()));
}

#[test]
fn base_name_survives_final_and_same_line_generation_reassembly() {
    let mut machine = machine();
    let mut provider = BuiltinItemLoadProvider::new(data());
    machine
        .apply_text("Rarity: NORMAL\nGold Ring", &mut provider)
        .unwrap();
    let parsed = machine.assembly_progress().unwrap().clone();
    assert_eq!(text(&parsed, "baseName"), Some("Gold Ring"));
    assert_base_name(&parsed, &machine);
    let base = parsed
        .field(parsed.root(), "base")
        .unwrap()
        .as_table()
        .unwrap();
    let lines = parsed
        .field(parsed.root(), "explicitModLines")
        .unwrap()
        .as_table()
        .unwrap();

    machine.finish_load(&mut provider).unwrap();
    let finished = machine.assembled().unwrap().clone();
    assert_base_name(&finished, &machine);
    assert_eq!(
        finished.field(finished.root(), "base"),
        Some(&Value::Table(base))
    );
    assert_eq!(
        finished.field(finished.root(), "explicitModLines"),
        Some(&Value::Table(lines))
    );

    // A range write invalidates completion, even when its row index is absent.
    machine.apply_mod_range(Some("999"), Some("0.1")).unwrap();
    assert!(machine.assembled().is_none());
    machine.finish_load(&mut provider).unwrap();
    let repeated = machine.assembled().unwrap();
    assert_base_name(repeated, &machine);
    assert_eq!(
        repeated.field(repeated.root(), "base"),
        Some(&Value::Table(base))
    );
    assert!(!finished.shares_storage_with(repeated));
    assert_eq!(text(&parsed, "baseName"), Some("Gold Ring"));
    assert_eq!(text(&finished, "baseName"), Some("Gold Ring"));
}

#[test]
fn reparse_replaces_stale_projection_then_no_base_preserves_loader_identity() {
    assert!(data().item_loading().base("Gold Ring").is_some());
    assert!(data().item_loading().base("Iron Ring").is_some());
    let mut machine = machine();
    let mut provider = BuiltinItemLoadProvider::new(data());
    machine
        .apply_text("Rarity: NORMAL\nGold Ring", &mut provider)
        .unwrap();
    machine.finish_load(&mut provider).unwrap();
    let first = machine.assembled().unwrap().clone();
    assert_eq!(
        machine.state().retained_fields.get("baseName"),
        Some(&ItemScalar::Text("Gold Ring".into()))
    );

    // The previous projection still names Gold Ring when ParseRaw selects Iron
    // Ring. Hydration must take the new dedicated field after replaying it.
    machine
        .apply_text("Rarity: NORMAL\nIron Ring", &mut provider)
        .unwrap();
    machine.finish_load(&mut provider).unwrap();
    let second = machine.assembled().unwrap().clone();
    assert_eq!(text(&second, "baseName"), Some("Iron Ring"));
    assert_base_name(&second, &machine);
    assert_eq!(
        machine.state().retained_fields.get("baseName"),
        Some(&ItemScalar::Text("Iron Ring".into()))
    );
    assert_eq!(text(&first, "baseName"), Some("Gold Ring"));

    machine
        .apply_text("No matching base", &mut provider)
        .unwrap();
    assert_eq!(machine.status(), ItemLoadStatus::NoBase);
    assert!(!machine.state().base_present);
    assert_eq!(machine.state().base_name.as_deref(), Some("Iron Ring"));
    assert!(machine.assembled().is_none());
    let no_base = machine.assembly_progress().unwrap().clone();
    assert!(!no_base.is_complete());
    assert!(no_base.field(no_base.root(), "base").is_none());
    assert_base_name(&no_base, &machine);
    assert!(second.field(second.root(), "base").is_some());
    assert_eq!(text(&second, "baseName"), Some("Iron Ring"));

    machine
        .apply_text("Rarity: NORMAL\nGold Ring", &mut provider)
        .unwrap();
    machine.finish_load(&mut provider).unwrap();
    assert_eq!(
        text(machine.assembled().unwrap(), "baseName"),
        Some("Gold Ring")
    );
    assert_eq!(text(&no_base, "baseName"), Some("Iron Ring"));
}

#[test]
fn missing_dedicated_base_name_removes_a_stale_scalar_without_promoting_no_base() {
    #[derive(Default)]
    struct Capture(Option<AssemblyRequest>);
    impl ItemLoadProvider for Capture {
        fn parse_modifier(&mut self, _: &ParseRequest) -> DependencyResult<ParseOutcome> {
            DependencyResult::Available(ParseOutcome {
                modifiers: Some(Vec::new()),
                extra: None,
            })
        }
        fn assemble_with_trace(&mut self, request: &AssemblyRequest) -> AssemblyExecution {
            self.0 = Some(request.clone());
            AssemblyExecution {
                outcome: DependencyResult::Unavailable("capture hydration input".into()),
                prefix: None,
            }
        }
    }
    let mut machine = machine();
    let mut capture = Capture::default();
    machine
        .apply_text("Rarity: NORMAL\nGold Ring", &mut capture)
        .unwrap();
    let mut request = capture.0.unwrap();
    // An authored finite input exercises the optional field independently of
    // ParseRaw's normal preservation rule and of the catalog's base names.
    request.state.base_present = false;
    request.state.base_name = Some("retained custom base \u{03bb}".into());
    request.reparsed = true;
    let previous = finish_no_base_jewel_radius(data().item_loading(), &request, false)
        .result
        .unwrap();
    assert_eq!(
        text(&previous, "baseName"),
        Some("retained custom base \u{03bb}")
    );
    request.state.retained_fields.insert(
        "baseName".into(),
        ItemScalar::Text("obsolete projection".into()),
    );
    request.state.base_name = None;
    request.previous = Some(previous.clone());
    let removed = finish_no_base_jewel_radius(data().item_loading(), &request, false)
        .result
        .unwrap();
    assert!(!removed.is_complete());
    assert!(
        removed
            .binding()
            .unwrap()
            .matches_attempt(request.binding())
    );
    assert!(removed.field(removed.root(), "baseName").is_none());
    assert_eq!(
        text(&previous, "baseName"),
        Some("retained custom base \u{03bb}")
    );

    // The same missing value is nil on fresh hydration, not a stale fallback.
    let mut arena = Arena::new(AssemblyLimits::default());
    let root = arena.new_table().unwrap();
    hydrate(
        &mut arena,
        root,
        &request,
        data().item_loading(),
        None,
        false,
    )
    .unwrap();
    assert_eq!(arena.get_field(root, "baseName").unwrap(), Value::Nil);
}
