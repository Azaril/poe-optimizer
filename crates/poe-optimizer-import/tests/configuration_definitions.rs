use poe_optimizer_core::options::Scalar;
use poe_optimizer_data::game_data::{
    GameDataLoader, GameDataPackage, LoadLimits, TrustPolicy, bundled_snapshot,
};
use poe_optimizer_import::{
    configuration,
    configuration_definitions::{DefinitionRecord, lookup_definitions},
};

fn snapshot(mut package: GameDataPackage) -> poe_optimizer_data::game_data::GameDataSnapshot {
    package.refresh_section_digests().unwrap();
    GameDataLoader::from_bytes(
        &package.canonical_bytes().unwrap(),
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
    .unwrap()
}
fn wrap(records: &str) -> String {
    format!(
        "<PathOfBuilding2><Config activeConfigSet='1'><ConfigSet id='1'>{records}</ConfigSet></Config></PathOfBuilding2>"
    )
}
#[test]
fn authored_kinds_options_and_unknowns_do_not_become_effective_configuration() {
    let xml = wrap(
        "<Input name='enemyIsBoss' string='Unlisted caller choice'/><Placeholder name='enemyIsBoss' number='42'/><Input name='callerOnlyKey' string='line1\n\tline2'/><CustomModifierBlock enabled='false'>+12 to Life</CustomModifierBlock><Future caller='retained'/>",
    );
    let source = configuration::project_xml(&xml).unwrap();
    let data = bundled_snapshot().unwrap();
    let result = lookup_definitions(&source, &data).unwrap();
    assert_eq!(result.source_xml_sha256(), source.source_sha256());
    assert_eq!(result.data(), data.identity());
    let records = result.sets()[0].records();
    let DefinitionRecord::Input { lookup, .. } = &records[0] else {
        panic!()
    };
    assert_eq!(
        lookup.value(),
        &Scalar::Text("Unlisted caller choice".into())
    );
    assert!(!lookup.is_unknown_key());
    assert!(
        lookup
            .definitions()
            .iter()
            .all(|d| d.scalar_kind_matches() && d.option_indices() == Some([].as_slice()))
    );
    let DefinitionRecord::Placeholder { lookup, .. } = &records[1] else {
        panic!()
    };
    assert!(
        lookup
            .definitions()
            .iter()
            .all(|d| !d.scalar_kind_matches())
    );
    let DefinitionRecord::Input { lookup, .. } = &records[2] else {
        panic!()
    };
    assert!(lookup.is_unknown_key());
    assert_eq!(lookup.value(), &Scalar::Text("line1\n\tline2".into()));
    assert!(matches!(records[3], DefinitionRecord::Block { .. }));
    assert!(matches!(records[4], DefinitionRecord::Unknown { .. }));
    let diagnostic = serde_json::to_value(&result).unwrap();
    assert_eq!(diagnostic["effective_configuration"], "not_evaluated");
    assert_eq!(diagnostic["game_mechanics"], "not_evaluated");
    assert_eq!(diagnostic["callbacks"], "not_executed");
    assert_eq!(diagnostic["option_membership_is_import_whitelist"], false);
    assert_eq!(source.source_xml(), xml);
}
#[test]
fn injected_keys_options_and_initial_defaults_drive_lookup_and_data_identity() {
    let base = bundled_snapshot().unwrap();
    let mut package = base.package().clone();
    let renamed = "callerRenamedDefinition";
    let row = package
        .configuration
        .definitions
        .iter_mut()
        .find(|d| d.key == "enemyIsBoss")
        .unwrap();
    row.key = renamed.into();
    row.options[0].value = Scalar::Text("caller\n\tchoice".into());
    row.defaults.input = None;
    row.defaults.option_index = Some(row.options[0].index);
    let selected_option = row.options[0].index;
    let changed = snapshot(package);
    assert_ne!(changed.identity(), base.identity());
    let xml = wrap(&format!(
        "<Input name='{renamed}' string='caller\n\tchoice'/><Input name='enemyIsBoss' string='None'/>"
    ));
    let source = configuration::project_xml(&xml).unwrap();
    let result = lookup_definitions(&source, &changed).unwrap();
    let DefinitionRecord::Input { lookup, .. } = &result.sets()[0].records()[0] else {
        panic!()
    };
    assert_eq!(
        lookup.definitions()[0].option_indices(),
        Some([selected_option].as_slice())
    );
    assert_eq!(
        result.initial_defaults_before_load().inputs[renamed],
        Scalar::Text("caller\n\tchoice".into())
    );
    let DefinitionRecord::Input { lookup, .. } = &result.sets()[0].records()[1] else {
        panic!()
    };
    assert!(lookup.is_unknown_key());
    assert_eq!(result.data(), changed.identity());
    let original_lookup = lookup_definitions(&source, &base).unwrap();
    let DefinitionRecord::Input { lookup, .. } = &original_lookup.sets()[0].records()[0] else {
        panic!()
    };
    assert!(lookup.is_unknown_key());
}
#[test]
fn duplicate_definitions_and_inactive_authored_sets_remain_ordered_and_distinct() {
    let data = bundled_snapshot().unwrap();
    let key = "conditionEnemyExitedPresenceRecently";
    let expected: Vec<_> = data
        .configuration()
        .definitions_for_key(key)
        .map(|d| d.id.as_str())
        .collect();
    assert_eq!(expected.len(), 2);
    let xml = format!(
        "<PathOfBuilding2><Config activeConfigSet='2'><ConfigSet id='1'><Input name='{key}' boolean='true'/></ConfigSet><ConfigSet id='2'><Placeholder name='callerHint' string='hint'/><Input name='{key}' boolean='false'/></ConfigSet></Config></PathOfBuilding2>"
    );
    let source = configuration::project_xml(&xml).unwrap();
    let result = lookup_definitions(&source, &data).unwrap();
    assert!(!result.sets()[0].is_selected());
    assert!(result.sets()[1].is_selected());
    for (set, index) in [(0, 0), (1, 1)] {
        let DefinitionRecord::Input { lookup, .. } = &result.sets()[set].records()[index] else {
            panic!()
        };
        assert_eq!(
            lookup
                .definitions()
                .iter()
                .map(|d| d.definition_id())
                .collect::<Vec<_>>(),
            expected
        );
    }
}
#[test]
fn all_five_originals_lookup_every_scalar_without_rewriting_source_or_admitting_mechanics() {
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/builds/breadth-20260908");
    let data = bundled_snapshot().unwrap();
    let mut inputs = 0;
    let mut placeholders = 0;
    for line in 1..=5 {
        let xml = std::fs::read_to_string(folder.join(format!("build-{line:02}.xml"))).unwrap();
        let source = configuration::project_xml(&xml).unwrap();
        let result = lookup_definitions(&source, &data).unwrap();
        assert_eq!(result.source_xml_sha256(), source.source_sha256());
        for set in result.sets() {
            for record in set.records() {
                let lookup = match record {
                    DefinitionRecord::Input { lookup, .. } => {
                        inputs += 1;
                        lookup
                    }
                    DefinitionRecord::Placeholder { lookup, .. } => {
                        placeholders += 1;
                        lookup
                    }
                    _ => continue,
                };
                assert!(!lookup.is_unknown_key(), "line {line}: {}", lookup.name());
                assert!(lookup.definitions().iter().all(|d| d.scalar_kind_matches()));
                for definition in lookup.definitions() {
                    if let Some(options) = definition.option_indices() {
                        assert!(!options.is_empty());
                    }
                }
            }
        }
        assert_eq!(source.source_xml(), xml);
    }
    assert_eq!((inputs, placeholders), (58, 170));
}
#[test]
fn custom_duplicate_catalog_expansion_is_bounded_before_publishing_a_partial_report() {
    let base = bundled_snapshot().unwrap();
    let mut package = base.package().clone();
    for row in &mut package.configuration.definitions {
        row.key = "sharedCallerKey".into();
    }
    let data = snapshot(package);
    let mut xml = "<PathOfBuilding2><Config activeConfigSet='1'>".to_string();
    for id in 1..=64 {
        xml.push_str(&format!("<ConfigSet id='{id}'><Input name='sharedCallerKey' boolean='true'/><Placeholder name='sharedCallerKey' number='1'/></ConfigSet>"));
    }
    xml.push_str("</Config></PathOfBuilding2>");
    let source = configuration::project_xml(&xml).unwrap();
    assert!(
        lookup_definitions(&source, &data)
            .unwrap_err()
            .to_string()
            .contains("expansion exceeds limit")
    );
}

#[test]
fn repeated_list_options_cannot_expand_lookup_work_or_output_without_bound() {
    let mut package = bundled_snapshot().unwrap().package().clone();
    let row = package
        .configuration
        .definitions
        .iter_mut()
        .find(|d| d.key == "enemyIsBoss")
        .unwrap();
    let option = row.options[0].clone();
    row.options = (1..=4096)
        .map(|index| {
            let mut option = option.clone();
            option.index = index;
            option
        })
        .collect();
    let data = snapshot(package);
    let value = data
        .configuration()
        .definitions_for_key("enemyIsBoss")
        .next()
        .unwrap()
        .options[0]
        .value
        .clone();
    let Scalar::Text(value) = value else { panic!() };
    let sets = (1..=17)
        .map(|id| {
            format!("<ConfigSet id='{id}'><Input name='enemyIsBoss' string='{value}'/></ConfigSet>")
        })
        .collect::<String>();
    let xml =
        format!("<PathOfBuilding2><Config activeConfigSet='1'>{sets}</Config></PathOfBuilding2>");
    let source = configuration::project_xml(&xml).unwrap();
    assert!(
        lookup_definitions(&source, &data)
            .unwrap_err()
            .to_string()
            .contains("option comparison expansion")
    );
}
