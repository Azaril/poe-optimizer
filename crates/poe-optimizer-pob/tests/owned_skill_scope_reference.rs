//! Original skill-loading observations. These do not emulate CalcSetup, grant
//! support applicability, or equate global-effect switches with weapon loadouts.
#![cfg(not(target_arch = "wasm32"))]
use mlua::Table;
use sha2::{Digest, Sha256};
#[path = "support/skill_preparation_source.rs"]
#[allow(dead_code)]
mod source_runtime;

fn fields(record: &Table) -> Table {
    record.get("fields").unwrap()
}

#[test]
fn all_original_source_free_groups_have_no_equipment_slot_binding() {
    let oracle = source_runtime::Oracle::new(false);
    let directory = source_runtime::repository().join("tests/fixtures/builds/breadth-20260908");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("index.json")).unwrap()).unwrap();
    let expected = [
        (19, 62, 15, 58, 1),
        (84, 174, 66, 156, 12),
        (14, 62, 11, 59, 0),
        (15, 62, 13, 60, 1),
        (68, 181, 53, 166, 6),
    ];
    for (index, expected) in expected.into_iter().enumerate() {
        let xml =
            std::fs::read_to_string(directory.join(format!("build-{:02}.xml", index + 1))).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(xml.as_bytes())),
            manifest["builds"][index]["xml_sha256"].as_str().unwrap(),
            "retain exact original fixture"
        );
        let observation = oracle.observe(&xml, None);
        assert!(observation.get::<bool>("success").unwrap());
        let groups: Table = observation.get("groups").unwrap();
        let mut counts = (0, 0, 0, 0, 0);
        for group in groups.sequence_values::<Table>().map(Result::unwrap) {
            counts.0 += 1;
            assert!(group.get::<bool>("attached").unwrap());
            let group_fields = fields(&group);
            assert!(group_fields.get::<bool>("enabled").unwrap());
            let source: Option<String> = group_fields.get("source").unwrap();
            let slot: Option<String> = group_fields.get("slot").unwrap();
            let gems: Table = group.get("gems").unwrap();
            counts.1 += gems.raw_len();
            if source.is_none() {
                counts.2 += 1;
                assert_eq!(slot, None, "source-free original has no saved slot");
                counts.3 += gems.raw_len();
                for gem in gems.sequence_values::<Table>().map(Result::unwrap) {
                    let gem_fields = fields(&gem);
                    assert!(gem_fields.get::<bool>("enabled").unwrap());
                    assert!(gem_fields.get::<bool>("enableGlobal1").unwrap());
                    assert!(gem_fields.get::<bool>("enableGlobal2").unwrap());
                }
            } else if let (Some(source), Some(slot)) = (source, slot) {
                assert!(source.starts_with("Item:"));
                assert!(matches!(slot.as_str(), "Weapon 1" | "Weapon 2 Swap"));
                counts.4 += 1;
            }
        }
        assert_eq!(counts, expected, "original {}", index + 1);
    }
}

#[test]
fn original_load_keeps_slot_source_enabled_and_global_effect_flags_distinct() {
    for warm in [false, true] {
        let oracle = source_runtime::Oracle::new(warm);
        // The final two booleans select effects in grantedEffectList, not weapon
        // sets. The whole original loader is executed, including defaults.
        for (group_input, gem_input, enabled, gem_enabled, globals, slot, source) in [
            ("", "", false, true, [true, false], None, None),
            (
                "enabled=\"false\" active=\"true\" slot=\"Weapon 1\"",
                "enableGlobal1=\"false\" enableGlobal2=\"true\"",
                true,
                true,
                [false, true],
                Some("Weapon 1"),
                None,
            ),
            (
                "enabled=\"true\" slot=\"Weapon 2 Swap\"",
                "enabled=\"false\" enableGlobal1=\"true\" enableGlobal2=\"false\"",
                true,
                false,
                [true, false],
                Some("Weapon 2 Swap"),
                None,
            ),
            (
                "enabled=\"true\" slot=\"\" source=\"Tree:123\"",
                "enableGlobal1=\"nil\" enableGlobal2=\"nil\"",
                true,
                true,
                [false, false],
                Some(""),
                Some("Tree:123"),
            ),
            (
                "enabled=\"TRUE\" slot=\"Unknown Slot\"",
                "enabled=\"TRUE\" enabledGlobal1=\"false\" enabledGlobal2=\"true\"",
                false,
                false,
                [true, false],
                Some("Unknown Slot"),
                None,
            ),
        ] {
            let xml = format!(
                "<Skills activeSkillSet=\"1\"><SkillSet id=\"1\"><Skill {group_input}><Gem skillId=\"SparkPlayer\" level=\"20\" quality=\"0\" {gem_input}/></Skill></SkillSet></Skills>"
            );
            let result = oracle.observe(&xml, None);
            assert!(result.get::<bool>("success").unwrap(), "{xml}");
            let groups: Table = result.get("groups").unwrap();
            assert_eq!(groups.raw_len(), 1);
            let group: Table = groups.get(1).unwrap();
            let group_fields = fields(&group);
            assert_eq!(group_fields.get::<bool>("enabled").unwrap(), enabled);
            assert_eq!(
                group_fields
                    .get::<Option<String>>("slot")
                    .unwrap()
                    .as_deref(),
                slot
            );
            assert_eq!(
                group_fields
                    .get::<Option<String>>("source")
                    .unwrap()
                    .as_deref(),
                source
            );
            let gems: Table = group.get("gems").unwrap();
            assert_eq!(gems.raw_len(), 1);
            let gem_fields = fields(&gems.get::<Table>(1).unwrap());
            assert_eq!(gem_fields.get::<bool>("enabled").unwrap(), gem_enabled);
            assert_eq!(gem_fields.get::<bool>("enableGlobal1").unwrap(), globals[0]);
            assert_eq!(gem_fields.get::<bool>("enableGlobal2").unwrap(), globals[1]);
        }
    }
}
