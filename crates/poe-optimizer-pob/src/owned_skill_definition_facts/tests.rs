use super::*;
use std::{path::PathBuf, sync::OnceLock};

const SNIPER: &str = "SummonSkeletalSnipersPlayer";
const ABYSSAL: &str = "TriggeredAbyssalApparitionPlayer";
const LATE: &str = "src/Data/Skills/sup_int.lua";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2")
}
fn authenticated() -> &'static AuthenticatedSkillDefinitionFactsExport {
    static EXPORT: OnceLock<AuthenticatedSkillDefinitionFactsExport> = OnceLock::new();
    EXPORT.get_or_init(|| export_owned_skill_definition_facts(&root(), Default::default()).unwrap())
}
fn sources() -> &'static BTreeMap<String, String> {
    static SOURCES: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    SOURCES.get_or_init(|| read_sources(&root(), Default::default()).unwrap())
}
fn late_mutation(code: &str) -> BTreeMap<String, String> {
    let mut changed = sources().clone();
    let original = changed.get(LATE).unwrap();
    // Keep the actual module body and call its returned constructor, then write
    // through its real shared skills argument. No declaration parser is involved.
    let wrapper = format!(
        "local original = (function()\n{original}\nend)()\nreturn function(...)\noriginal(...)\nlocal skills = select(1, ...)\n{code}\nend\n"
    );
    changed.insert(LATE.into(), wrapper);
    changed
}
fn fact<'a>(rows: &'a [SkillDefinitionFact], id: &str) -> &'a SkillDefinitionFact {
    rows.iter().find(|row| row.id == id).unwrap()
}

#[test]
fn verified_final_catalog_preserves_exact_identities_absence_and_positive_source_fact() {
    let export = authenticated();
    assert_eq!(export.definition(SNIPER).unwrap().cannot_be_supported, None);
    assert_eq!(
        export.definition(ABYSSAL).unwrap().cannot_be_supported,
        Some(true)
    );
    assert!(
        export
            .definition("not-a-source-definition")
            .unwrap_err()
            .to_string()
            .contains("missing skill definition")
    );
    assert!(
        export
            .catalog()
            .skills
            .windows(2)
            .all(|pair| pair[0].id < pair[1].id)
    );
    assert_eq!(export.evidence().definitions, export.catalog().skills.len());
    assert_eq!(
        export.evidence().definitions,
        export.evidence().absent
            + export.evidence().explicit_false
            + export.evidence().explicit_true
    );
    assert_eq!(
        export.evidence().source_manifest_sha256,
        source::manifest_sha256()
    );
    assert_eq!(export.evidence().extraction_source_files.len(), PATHS.len());
    assert_eq!(
        export.catalog().source.files,
        export.evidence().extraction_source_files
    );
    for pin in &export.catalog().source.files {
        assert_eq!(pin.sha256, source::expected_file_sha256(&pin.path).unwrap());
    }
    let order = &export.evidence().module_order;
    assert!(
        order
            .iter()
            .position(|p| p == "src/Data/Skills/act_int.lua")
            .unwrap()
            < order.iter().position(|p| p == LATE).unwrap()
    );
    assert!(order.iter().all(|path| sources().contains_key(path)));
    assert_eq!(
        export.evidence().catalog_sha256,
        game_data::hash(export.catalog_bytes())
    );
    let decoded: SkillDefinitionFactsCatalog =
        serde_json::from_slice(export.catalog_bytes()).unwrap();
    export.validate_catalog(&decoded).unwrap();
    assert_eq!(
        catalog_bytes(&decoded, export.catalog_bytes().len()).unwrap(),
        export.catalog_bytes()
    );
    let json: serde_json::Value = serde_json::from_slice(export.catalog_bytes()).unwrap();
    assert!(
        json["skills"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == SNIPER)
            .unwrap()["cannot_be_supported"]
            .is_null()
    );
    for bad in [
        r#"{"id":"x"}"#,
        r#"{"id":"x","cannot_be_supported":0}"#,
        r#"{"id":"x","cannot_be_supported":false,"extra":true}"#,
    ] {
        assert!(serde_json::from_str::<SkillDefinitionFact>(bad).is_err());
    }
}

#[test]
fn late_module_writes_win_without_nil_or_false_fallbacks() {
    let baseline = authenticated();
    for (value, expected) in [("false", Some(false)), ("true", Some(true)), ("nil", None)] {
        let changed = late_mutation(&format!("skills[{SNIPER:?}].cannotBeSupported = {value}"));
        let (rows, order) = construct_facts(&changed, Default::default()).unwrap();
        assert_eq!(fact(&rows, SNIPER).cannot_be_supported, expected);
        assert_eq!(fact(&rows, ABYSSAL), baseline.definition(ABYSSAL).unwrap());
        assert_eq!(rows.len(), baseline.catalog().skills.len());
        assert_eq!(&order, &baseline.evidence().module_order);
        let mut candidate = baseline.catalog().clone();
        candidate.skills = rows;
        if expected.is_some() {
            assert!(baseline.validate_catalog(&candidate).is_err());
        } else {
            baseline.validate_catalog(&candidate).unwrap();
        }
    }
    // Removal cannot become a negative fact: the normal gem constructor requires
    // this exact definition, and the complete capture must fail atomically.
    assert!(
        construct_facts(
            &late_mutation(&format!("skills[{SNIPER:?}] = nil")),
            Default::default()
        )
        .is_err()
    );
}

#[test]
fn late_new_non_gem_definition_and_whole_row_replacement_are_captured() {
    let baseline = authenticated();
    let added_id = "AcquiredLateNonGemSkill";
    assert!(baseline.definition(added_id).is_err());
    let changed = late_mutation(&format!(
        "skills[{added_id:?}] = copyTable(skills[{SNIPER:?}])\n\
         skills[{added_id:?}].cannotBeSupported = false"
    ));
    let (mut rows, order) = construct_facts(&changed, Default::default()).unwrap();
    assert_eq!(rows.len(), baseline.catalog().skills.len() + 1);
    assert_eq!(
        fact(&rows, added_id),
        &SkillDefinitionFact {
            id: added_id.into(),
            cannot_be_supported: Some(false),
        }
    );
    rows.retain(|row| row.id != added_id);
    assert_eq!(rows, baseline.catalog().skills);
    assert_eq!(&order, &baseline.evidence().module_order);

    let changed = late_mutation(&format!(
        "local replacement = copyTable(skills[{SNIPER:?}])\n\
         replacement.cannotBeSupported = true\n\
         skills[{SNIPER:?}] = replacement"
    ));
    let (rows, order) = construct_facts(&changed, Default::default()).unwrap();
    let mut expected = baseline.catalog().skills.clone();
    expected
        .iter_mut()
        .find(|row| row.id == SNIPER)
        .unwrap()
        .cannot_be_supported = Some(true);
    assert_eq!(rows, expected);
    assert_eq!(&order, &baseline.evidence().module_order);
}

#[test]
fn wrong_field_types_and_metatables_are_not_interpreted_as_boolean_facts() {
    // Establish the real constructor baseline before testing rejection paths.
    assert_eq!(
        authenticated()
            .definition(SNIPER)
            .unwrap()
            .cannot_be_supported,
        None
    );
    for value in ["0", "1", "'false'", "{}", "function() return false end"] {
        let changed = late_mutation(&format!("skills[{SNIPER:?}].cannotBeSupported = {value}"));
        let error = construct_facts(&changed, Default::default())
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("cannotBeSupported must be Boolean or absent"),
            "{error}"
        );
    }
    for expression in ["data", "skills", &format!("skills[{SNIPER:?}]")] {
        let changed = late_mutation(&format!("setmetatable({expression}, {{}})"));
        let error = construct_facts(&changed, Default::default())
            .unwrap_err()
            .to_string();
        assert!(error.contains("must not have a metatable"), "{error}");
    }
}

#[test]
fn exact_row_identity_is_required_independently_of_field_absence() {
    let lua = mlua::Lua::new();
    let skills: Table = lua
        .load("return { source = { id = 'source' } }")
        .eval()
        .unwrap();
    assert_eq!(
        project(&skills, Default::default()).unwrap(),
        vec![SkillDefinitionFact {
            id: "source".into(),
            cannot_be_supported: None
        }]
    );
    let row: Table = skills.raw_get("source").unwrap();
    for value in [
        Value::Nil,
        Value::Boolean(false),
        Value::String(lua.create_string("other").unwrap()),
    ] {
        row.raw_set("id", value).unwrap();
        assert!(project(&skills, Default::default()).is_err());
    }
    row.raw_set("id", "source").unwrap();
    skills.raw_set(3, lua.create_table().unwrap()).unwrap();
    assert!(
        project(&skills, Default::default())
            .unwrap_err()
            .to_string()
            .contains("skill key must be text")
    );
}

#[test]
fn acquisition_is_deterministic_and_copied_provenance_cannot_authorize_changes() {
    let first = authenticated();
    let second = export_owned_skill_definition_facts(&root(), Default::default()).unwrap();
    assert_eq!(first.catalog_bytes(), second.catalog_bytes());
    assert_eq!(first.evidence(), second.evidence());
    for change in 0..5 {
        let mut candidate = first.catalog().clone();
        match change {
            0 => {
                candidate.skills[0].cannot_be_supported =
                    Some(!candidate.skills[0].cannot_be_supported.unwrap_or(false))
            }
            1 => candidate.skills[0].id.push_str("-changed"),
            2 => {
                candidate.skills.pop();
            }
            3 => candidate.skills.swap(0, 1),
            _ => candidate.source.files[0].sha256 = "0".repeat(64),
        }
        assert!(
            first
                .validate_catalog(&candidate)
                .unwrap_err()
                .to_string()
                .contains("independently authenticated")
        );
    }
    let missing = tempfile::tempdir().unwrap();
    std::fs::write(missing.path().join("catalog.json"), first.catalog_bytes()).unwrap();
    assert!(export_owned_skill_definition_facts(missing.path(), Default::default()).is_err());
}

#[test]
fn finite_projection_and_serialization_obey_exact_tightened_limits() {
    let export = authenticated();
    let maximum_text = export
        .catalog()
        .skills
        .iter()
        .map(|row| row.id.len())
        .max()
        .unwrap();
    let limits = SkillDefinitionFactsExportLimits {
        max_source_bytes: sources().values().map(String::len).sum(),
        max_definitions: export.catalog().skills.len(),
        max_text_bytes: maximum_text,
        max_catalog_bytes: export.catalog_bytes().len(),
    };
    let exact = export_owned_skill_definition_facts(&root(), limits).unwrap();
    assert_eq!(exact.catalog_bytes(), export.catalog_bytes());
    assert_eq!(exact.evidence(), export.evidence());
    for (limits, expected) in [
        (
            SkillDefinitionFactsExportLimits {
                max_source_bytes: limits.max_source_bytes - 1,
                ..limits
            },
            "source byte limit",
        ),
        (
            SkillDefinitionFactsExportLimits {
                max_definitions: limits.max_definitions - 1,
                ..limits
            },
            "definition count limit",
        ),
        (
            SkillDefinitionFactsExportLimits {
                max_text_bytes: limits.max_text_bytes - 1,
                ..limits
            },
            "text byte limit",
        ),
        (
            SkillDefinitionFactsExportLimits {
                max_catalog_bytes: limits.max_catalog_bytes - 1,
                ..limits
            },
            "catalog byte limit",
        ),
    ] {
        let error = export_owned_skill_definition_facts(&root(), limits)
            .unwrap_err()
            .to_string();
        assert!(error.contains(expected), "{error}");
    }
    for limits in [
        SkillDefinitionFactsExportLimits {
            max_source_bytes: 0,
            ..limits
        },
        SkillDefinitionFactsExportLimits {
            max_definitions: usize::MAX,
            ..limits
        },
        SkillDefinitionFactsExportLimits {
            max_text_bytes: 0,
            ..limits
        },
        SkillDefinitionFactsExportLimits {
            max_catalog_bytes: usize::MAX,
            ..limits
        },
    ] {
        assert!(
            export_owned_skill_definition_facts(&root(), limits)
                .unwrap_err()
                .to_string()
                .contains("invalid")
        );
    }
    assert_eq!(export.catalog_bytes(), exact.catalog_bytes());
}
