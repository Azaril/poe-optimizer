//! NoBase executes only the ParseRaw radius tail and cannot register an item.
use poe_optimizer_data::{
    game_data::{GameDataSnapshot, bundled_snapshot},
    item_loading::{ItemMetadataTable, ItemMetadataValue as V},
};
use poe_optimizer_import::item_loading::*;
use std::{collections::BTreeMap, sync::OnceLock};

fn data() -> &'static GameDataSnapshot {
    static DATA: OnceLock<GameDataSnapshot> = OnceLock::new();
    DATA.get_or_init(|| bundled_snapshot().unwrap())
}
fn machine() -> ItemLoadMachine<'static> {
    let mut m = ItemLoadMachine::new(data().item_loading());
    m.set_xml_attributes(&[("id".into(), "1".into())].into());
    m.set_jewel_radius_context(
        JewelRadiusContext::resolve(
            data().item_loading(),
            "0_5",
            JewelRadiusProvenance::ExplicitCaller,
        )
        .unwrap(),
    )
    .unwrap();
    m
}
fn index(m: &ItemLoadMachine<'_>) -> Option<f64> {
    match m.state().retained_fields.get("jewelRadiusIndex") {
        Some(ItemScalar::Number(n)) => n.value(),
        _ => None,
    }
}
struct Parser;
impl ItemLoadProvider for Parser {
    fn parse_modifier(&mut self, r: &ParseRequest) -> DependencyResult<ParseOutcome> {
        let (key, value) = match r.text.as_str() {
            "retained override" => ("timeLostJewelRadiusOverride", V::Number(5.0)),
            "retained table" => (
                "radiusIndex",
                V::Table(ItemMetadataTable {
                    fields: [("marker".into(), V::Number(7.0))].into(),
                    indexed: BTreeMap::new(),
                }),
            ),
            _ => {
                return DependencyResult::Available(ParseOutcome {
                    modifiers: Some(Vec::new()),
                    extra: None,
                });
            }
        };
        let payload = V::Table(ItemMetadataTable {
            fields: [("key".into(), V::Text(key.into())), ("value".into(), value)].into(),
            indexed: BTreeMap::new(),
        });
        DependencyResult::Available(ParseOutcome {
            extra: None,
            modifiers: Some(vec![ItemMetadataTable {
                fields: [
                    ("name".into(), V::Text("JewelData".into())),
                    ("type".into(), V::Text("LIST".into())),
                    ("value".into(), payload),
                    ("flags".into(), V::Number(0.0)),
                    ("keywordFlags".into(), V::Number(0.0)),
                ]
                .into(),
                indexed: BTreeMap::new(),
            }]),
        })
    }
}
#[test]
fn no_base_tail_overrides_new_header_without_reverting_the_new_parse_state() {
    let mut m = machine();
    let mut p = NativeItemLoadProvider::with_native_assembly(data(), Parser);
    m.apply_text(
        "Rarity: NORMAL\nRuby\nImplicits: 0\nretained override",
        &mut p,
    )
    .unwrap();
    m.finish_load(&mut p).unwrap();
    assert_eq!(index(&m), Some(5.0));
    let old = m.assembled().unwrap().clone();
    let jewel = old
        .field(old.root(), "jewelData")
        .unwrap()
        .as_table()
        .unwrap();
    let old_lines = old
        .field(old.root(), "explicitModLines")
        .unwrap()
        .as_table()
        .unwrap();
    assert!(old.field(old.root(), "base").is_some());
    m.apply_text(
        "Rarity: NORMAL\nDefinitely Not A Base\nRadius: Small\nArmour: 777",
        &mut p,
    )
    .unwrap();
    assert_eq!(m.status(), ItemLoadStatus::NoBase);
    assert!(!m.state().base_present);
    assert_eq!(
        index(&m),
        Some(5.0),
        "tail follows the Small header despite BuildModList returning early"
    );
    let current = m.assembly_progress().unwrap();
    assert!(!current.is_complete());
    assert!(current.field(current.root(), "base").is_none());
    assert_eq!(
        current.field(current.root(), "name").unwrap().as_str(),
        Some(m.state().name.as_str())
    );
    assert_eq!(
        current
            .field(current.root(), "jewelData")
            .unwrap()
            .as_table(),
        Some(jewel)
    );
    assert_ne!(
        current
            .field(current.root(), "explicitModLines")
            .unwrap()
            .as_table(),
        Some(old_lines)
    );
    let armour = current
        .field(current.root(), "armourData")
        .unwrap()
        .as_table()
        .unwrap();
    assert_eq!(
        current.field(armour, "Armour").unwrap().as_number(),
        Some(777.0)
    );
    let no_base = current.clone();
    m.finish_load(&mut p).unwrap();
    assert_eq!(m.status(), ItemLoadStatus::NoBase);
    assert!(m.assembled().is_none());
    assert!(m.assembly_progress().unwrap().shares_storage_with(&no_base));
    assert!(
        old.field(old.root(), "base").is_some(),
        "retained earlier generation stays immutable"
    );
    m.apply_text("Rarity: NORMAL\nGold Ring\nImplicits: 0", &mut p)
        .unwrap();
    m.finish_load(&mut p).unwrap();
    assert_eq!(m.status(), ItemLoadStatus::Complete);
    assert_eq!(
        index(&m),
        Some(5.0),
        "committed obsolete Small header must not replay on the next assembly"
    );
    let next = m.assembled().unwrap();
    let armour = next
        .field(next.root(), "armourData")
        .unwrap()
        .as_table()
        .unwrap();
    assert_eq!(
        next.field(armour, "Armour").unwrap().as_number(),
        Some(777.0)
    );
}
#[test]
fn no_base_variable_preserves_table_alias_and_later_concrete_header_replaces_it() {
    let mut m = machine();
    let mut p = NativeItemLoadProvider::with_native_assembly(data(), Parser);
    m.apply_text("Rarity: NORMAL\nRuby\nImplicits: 0\nretained table", &mut p)
        .unwrap();
    m.finish_load(&mut p).unwrap();
    let old = m.assembled().unwrap();
    let data_id = old
        .field(old.root(), "jewelData")
        .unwrap()
        .as_table()
        .unwrap();
    let index_id = old
        .field(data_id, "radiusIndex")
        .unwrap()
        .as_table()
        .unwrap();
    m.apply_text(
        "Rarity: NORMAL\nDefinitely Not A Base\nRadius: Medium\nRadius: Variable",
        &mut p,
    )
    .unwrap();
    assert_eq!(m.status(), ItemLoadStatus::NoBase);
    assert!(!m.state().retained_fields.contains_key("jewelRadiusIndex"));
    let item = m.assembly_progress().unwrap();
    assert!(!item.is_complete());
    assert_eq!(
        item.field(item.root(), "jewelRadiusIndex")
            .unwrap()
            .as_table(),
        Some(index_id)
    );
    assert_eq!(
        item.field(data_id, "radiusIndex").unwrap().as_table(),
        Some(index_id)
    );
    assert!(item.field(item.root(), "base").is_none());
    m.apply_text(
        "Rarity: NORMAL\nAnother Missing Base\nRadius: Small",
        &mut p,
    )
    .unwrap();
    assert_eq!(m.status(), ItemLoadStatus::NoBase);
    assert_eq!(index(&m), Some(1.0));
    let item = m.assembly_progress().unwrap();
    assert_eq!(
        item.field(item.root(), "jewelRadiusIndex")
            .unwrap()
            .as_number(),
        Some(1.0)
    );
    assert_eq!(
        item.field(data_id, "radiusIndex").unwrap().as_table(),
        Some(index_id)
    );
    assert!(m.assembled().is_none());
}
#[test]
fn absent_opaque_assembly_is_not_misrepresented_as_known_missing_jewel_data() {
    struct Opaque;
    impl ItemLoadProvider for Opaque {
        fn format_line(&mut self, request: &FormatRequest) -> DependencyResult<String> {
            DependencyResult::Available(request.text.clone())
        }
        fn parse_modifier(&mut self, _: &ParseRequest) -> DependencyResult<ParseOutcome> {
            DependencyResult::Available(ParseOutcome {
                modifiers: Some(Vec::new()),
                extra: None,
            })
        }
        fn assemble(&mut self, _: &AssemblyRequest) -> DependencyResult<AssemblyOutcome> {
            DependencyResult::Available(AssemblyOutcome {
                assembled: None,
                armour_data: ArmourDataUpdate::Preserve,
                modifier_payloads: None,
                requirements: None,
                state_updates: Default::default(),
                evidence: Default::default(),
            })
        }
    }
    let mut m = machine();
    let mut p = Opaque;
    m.apply_text("Rarity: NORMAL\nRuby\nImplicits: 0", &mut p)
        .unwrap();
    assert_eq!(m.status(), ItemLoadStatus::Complete);
    assert!(m.assembly_progress().is_none());
    m.apply_text("Rarity: NORMAL\nDefinitely Not A Base", &mut p)
        .unwrap();
    assert_eq!(m.status(), ItemLoadStatus::NoBase);
    assert!(m.assembled().is_none());
    m.apply_text(
        "Rarity: NORMAL\nDefinitely Not A Base\nRadius: Variable",
        &mut p,
    )
    .unwrap();
    assert_eq!(m.status(), ItemLoadStatus::Pending);
    assert!(
        m.pending()
            .unwrap()
            .message
            .contains("prior assembly's owned state"),
        "{:?}",
        m.pending()
    );
    assert!(m.assembled().is_none());
    assert!(
        matches!(m.state().retained_fields.get("jewelRadiusLabel"), Some(ItemScalar::Text(s)) if s == "Variable")
    );
}
