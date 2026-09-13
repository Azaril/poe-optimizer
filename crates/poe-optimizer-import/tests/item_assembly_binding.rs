use poe_optimizer_data::{
    game_data::{GameDataSnapshot, bundled_snapshot},
    item_loading::ItemMetadataTable,
};
use poe_optimizer_import::item_loading::*;
use std::{collections::BTreeMap, sync::OnceLock};
const RING: &str = "Rarity: NORMAL\nGold Ring";
fn data() -> &'static GameDataSnapshot {
    static DATA: OnceLock<GameDataSnapshot> = OnceLock::new();
    DATA.get_or_init(|| bundled_snapshot().unwrap())
}
fn machine() -> ItemLoadMachine<'static> {
    let mut m = ItemLoadMachine::new(data().item_loading());
    m.set_xml_attributes(&[("id".into(), "1".into())].into());
    m
}
fn outcome(item: Option<assembly::AssembledItem>) -> AssemblyOutcome {
    AssemblyOutcome {
        assembled: item,
        armour_data: ArmourDataUpdate::Preserve,
        modifier_payloads: None,
        requirements: None,
        state_updates: BTreeMap::new(),
        evidence: ItemMetadataTable::default(),
    }
}
struct Capture {
    inner: BuiltinItemLoadProvider<'static>,
    last: Option<AssemblyOutcome>,
}
impl ItemLoadProvider for Capture {
    fn parse_modifier(&mut self, r: &ParseRequest) -> DependencyResult<ParseOutcome> {
        self.inner.parse_modifier(r)
    }
    fn format_with_trace(&mut self, r: &FormatRequest) -> FormatOutcome {
        self.inner.format_with_trace(r)
    }
    fn lookup_unique(&mut self, r: &UniqueRequest) -> DependencyResult<Option<UniqueOutcome>> {
        self.inner.lookup_unique(r)
    }
    fn assemble_with_trace(&mut self, r: &AssemblyRequest) -> AssemblyExecution {
        let result = self.inner.assemble_with_trace(r);
        if let DependencyResult::Available(value) = &result.outcome {
            self.last = Some(value.clone());
        }
        result
    }
}
struct Replay {
    value: AssemblyOutcome,
    prefix: bool,
}
impl ItemLoadProvider for Replay {
    fn assemble_with_trace(&mut self, _: &AssemblyRequest) -> AssemblyExecution {
        if self.prefix {
            AssemblyExecution {
                outcome: DependencyResult::Unavailable("pending".into()),
                prefix: Some(self.value.clone()),
            }
        } else {
            AssemblyExecution {
                outcome: DependencyResult::Available(self.value.clone()),
                prefix: None,
            }
        }
    }
}
#[test]
fn diagnostic_completion_without_owned_output_cannot_authorize_registration() {
    let mut m = machine();
    let mut provider = Replay {
        value: outcome(None),
        prefix: false,
    };
    m.apply_text(RING, &mut provider).unwrap();
    m.finish_load(&mut provider).unwrap();
    assert_eq!(m.status(), ItemLoadStatus::Complete);
    assert!(m.assembled().is_none());
}
#[test]
fn completed_artifact_invalidates_on_range_attribute_and_reparse_writes() {
    let mut m = machine();
    let mut provider = BuiltinItemLoadProvider::new(data());
    m.apply_text(RING, &mut provider).unwrap();
    m.finish_load(&mut provider).unwrap();
    let first = m.assembled().unwrap().clone();
    m.apply_mod_range(Some("999"), Some("0.1")).unwrap();
    assert!(m.assembled().is_none());
    m.finish_load(&mut provider).unwrap();
    assert!(m.assembled().unwrap().is_complete());
    assert!(!first.shares_storage_with(m.assembled().unwrap()));
    m.set_xml_attributes(&[("id".into(), "2".into())].into());
    assert!(m.assembled().is_none());
    m.finish_load(&mut provider).unwrap();
    assert!(m.assembled().is_some());
    m.apply_text("No matching base", &mut provider).unwrap();
    assert!(m.assembled().is_none());
}
#[test]
fn replayed_success_from_same_item_previous_attempt_is_rejected() {
    let mut m = machine();
    let mut capture = Capture {
        inner: BuiltinItemLoadProvider::new(data()),
        last: None,
    };
    m.apply_text(RING, &mut capture).unwrap();
    m.finish_load(&mut capture).unwrap();
    let mut replay = Replay {
        value: capture.last.unwrap(),
        prefix: false,
    };
    let error = m.finish_load(&mut replay).unwrap_err();
    assert!(error.to_string().contains("foreign owned result"));
    assert!(m.assembled().is_none());
}
#[test]
fn foreign_success_and_failure_prefix_are_rejected_before_state_writes() {
    let mut source = machine();
    let mut provider = BuiltinItemLoadProvider::new(data());
    source.apply_text(RING, &mut provider).unwrap();
    source.finish_load(&mut provider).unwrap();
    for prefix in [false, true] {
        let mut target = machine();
        let mut value = outcome(Some(source.assembled().unwrap().clone()));
        value
            .state_updates
            .insert("injectedMarker".into(), ItemScalar::Boolean(true));
        let mut replay = Replay { value, prefix };
        let error = target.apply_text(RING, &mut replay).unwrap_err();
        assert!(error.to_string().contains("foreign owned result"));
        assert!(
            !target
                .state()
                .retained_fields
                .contains_key("injectedMarker")
        );
        assert!(target.assembled().is_none());
    }
}
#[test]
fn incomplete_native_artifact_cannot_be_replayed_as_success() {
    let mut source = machine();
    let mut provider = BuiltinItemLoadProvider::new(data());
    source
        .apply_text("Rarity: NORMAL\nSapphire", &mut provider)
        .unwrap();
    assert_eq!(source.status(), ItemLoadStatus::Pending);
    let item = source.assembly_progress().unwrap().clone();
    assert!(!item.is_complete());
    let mut target = machine();
    let mut replay = Replay {
        value: outcome(Some(item)),
        prefix: false,
    };
    assert!(
        target
            .apply_text(RING, &mut replay)
            .unwrap_err()
            .to_string()
            .contains("incomplete")
    );
    assert!(target.assembled().is_none());
}
