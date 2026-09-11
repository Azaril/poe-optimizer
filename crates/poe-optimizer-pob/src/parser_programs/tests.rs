use super::*;
use poe_optimizer_data::modifier_parser::*;
use std::sync::OnceLock;
fn fixture() -> &'static (ModifierParserCatalog, BTreeMap<String, String>) {
    static FIXTURE: OnceLock<(ModifierParserCatalog, BTreeMap<String, String>)> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let owner = poe_optimizer_data::game_data::bundled_snapshot()
            .unwrap()
            .modifier_parser()
            .clone();
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
        let sources = owner
            .data()
            .source
            .files
            .keys()
            .map(|path| {
                (
                    path.clone(),
                    crate::source::read_verified_text(&root, path).unwrap(),
                )
            })
            .collect();
        (owner, sources)
    })
}
#[test]
fn program_source_authentication_rejects_changed_missing_and_unmanifested_bytes() {
    let (owner, sources) = fixture();
    validate_sources(sources, owner).unwrap();
    let mut changed = sources.clone();
    changed
        .get_mut("src/Modules/ModParser.lua")
        .unwrap()
        .push_str("\nerror('must never execute')\n");
    let error = extract_from_sources(&changed, owner)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("not the authenticated normalized file"),
        "{error}"
    );
    let mut missing = sources.clone();
    missing.remove("src/Modules/ModParser.lua");
    assert!(
        extract_from_sources(&missing, owner)
            .unwrap_err()
            .to_string()
            .contains("missing program source dependency")
    );
    let mut extra = sources.clone();
    extra.insert(
        "src/unmanifested.lua".into(),
        "error('must never execute')".into(),
    );
    assert!(
        extract_from_sources(&extra, owner)
            .unwrap_err()
            .to_string()
            .contains("not manifested")
    );
    let mut unnormalized = sources.clone();
    let text = unnormalized.get_mut("src/Modules/ModParser.lua").unwrap();
    *text = text.replace('\n', "\r\n");
    assert!(
        extract_from_sources(&unnormalized, owner)
            .unwrap_err()
            .to_string()
            .contains("not the authenticated normalized file")
    );
}
#[test]
fn program_extraction_preserves_legacy_owner_and_inventories_every_other_callback() {
    let (owner, sources) = fixture();
    let before = serde_json::to_vec(owner.data()).unwrap();
    let result = extract_from_sources(sources, owner).unwrap();
    assert!(result.catalog().is_bound_to(owner));
    assert_eq!(serde_json::to_vec(owner.data()).unwrap(), before);
    assert_eq!(result.implementation_sha256().len(), 64);
    assert_eq!(
        result.catalog().data().callbacks.len(),
        result.catalog().data().programs.len()
    );
    for (index, _) in owner.data().callbacks.iter().enumerate() {
        let id = ParserCallbackId(index as u32 + 1);
        let legacy = matches!(
            owner.data().factories.get(&id),
            Some(ParserFactoryDisposition::Pure(_))
        );
        let program = result.catalog().data().callbacks.contains_key(&id);
        let unsupported = result.unsupported().contains_key(&id);
        assert_eq!(
            usize::from(legacy) + usize::from(program) + usize::from(unsupported),
            1,
            "callback {id:?}"
        );
    }
    // These IDs are source-fixture assertions, never production dispatch keys.
    for id in [15, 286, 738, 739] {
        assert!(
            result
                .catalog()
                .data()
                .callbacks
                .contains_key(&ParserCallbackId(id)),
            "required original fixture {id}: {:?}",
            result.unsupported().get(&ParserCallbackId(id))
        );
    }
}
#[test]
fn program_extraction_rejects_forged_lookup_owner_after_original_construction() {
    let (owner, sources) = fixture();
    let mut data = owner.data().clone();
    let table = data.dictionaries[&ParserDictionary::GemIdLookup];
    data.tables[table.0 as usize - 1].fields.insert(
        "test-only forged lookup".into(),
        ParserValue::Text("forged".into()),
    );
    let altered = ModifierParserCatalog::new(data).unwrap();
    let error = extract_from_sources(sources, &altered)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("owner differs from complete original parser extraction"),
        "{error}"
    );
}

#[test]
fn program_owner_authentication_retains_signed_zero_distinctions() {
    let (owner, sources) = fixture();
    let mut data = owner.data().clone();
    let value = data
        .tables
        .iter_mut()
        .flat_map(|table| table.fields.values_mut().chain(table.indexed.values_mut()))
        .find(|value| matches!(value, ParserValue::Number(number) if *number == 0.0))
        .expect("source zero fixture");
    let ParserValue::Number(number) = value else {
        unreachable!()
    };
    *number = -*number;
    let altered = ModifierParserCatalog::new(data).unwrap();
    // Ordinary floating equality would miss this; the source-bound representation must not.
    assert_eq!(owner.data(), altered.data());
    assert_ne!(
        serde_json::to_vec(owner.data()).unwrap(),
        serde_json::to_vec(altered.data()).unwrap()
    );
    let error = extract_from_sources(sources, &altered)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("owner differs from complete original parser extraction"),
        "{error}"
    );
}
