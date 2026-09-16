//! Finite actor acquisition checked against a separate literal inventory. The
//! comparison deliberately does not reuse the production Lua projection.
use poe_optimizer_import::owned_actor_baselines::{ActorBaselineCatalog, ActorScalarFact};
use poe_optimizer_pob::{
    owned_actor_baselines::{
        ActorBaselineExportLimits, AuthenticatedActorBaselineExport, export_owned_actor_baselines,
    },
    source,
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf, sync::OnceLock};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2")
}
fn export() -> &'static AuthenticatedActorBaselineExport {
    static RESULT: OnceLock<AuthenticatedActorBaselineExport> = OnceLock::new();
    RESULT.get_or_init(|| export_owned_actor_baselines(&root(), Default::default()).unwrap())
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
#[derive(Default)]
struct LiteralRow {
    module: String,
    numeric: BTreeMap<String, f64>,
    flags: BTreeMap<String, bool>,
    strings: BTreeMap<String, String>,
    skills: Vec<String>,
    modifiers: usize,
}
fn literal_inventory() -> BTreeMap<String, LiteralRow> {
    let mut result = BTreeMap::new();
    let mut assignments = 0;
    for module in ["src/Data/Minions.lua", "src/Data/Spectres.lua"] {
        let source = fs::read_to_string(root().join(module)).unwrap();
        let mut current: Option<(String, LiteralRow)> = None;
        let mut skills = false;
        for line in source.lines() {
            if let Some(key) = line
                .strip_prefix("minions[\"")
                .and_then(|v| v.strip_suffix("\"] = {"))
            {
                assert!(current.is_none());
                current = Some((
                    key.into(),
                    LiteralRow {
                        module: module.into(),
                        ..Default::default()
                    },
                ));
                assignments += 1;
                continue;
            }
            let Some((_, row)) = &mut current else {
                continue;
            };
            if line == "}" {
                let (key, mut row) = current.take().unwrap();
                if module.ends_with("Spectres.lua") {
                    row.strings
                        .insert("limit".into(), "ActiveSpectreLimit".into());
                }
                result.insert(key, row); // Final source assignment wins.
                continue;
            }
            if line == "\tskillList = {" {
                skills = true;
                continue;
            }
            if skills {
                if line == "\t}," {
                    skills = false;
                } else {
                    let value = line.trim().trim_end_matches(',');
                    row.skills.push(serde_json::from_str(value).unwrap());
                }
                continue;
            }
            if line.starts_with("\t\tmod(") || line.starts_with("\t\tflag(") {
                row.modifiers += 1;
            }
            let Some(line) = line.strip_prefix('\t').filter(|v| !v.starts_with('\t')) else {
                continue;
            };
            let Some((key, value)) = line.split_once(" = ") else {
                continue;
            };
            let value = value.trim_end_matches(',');
            if let Ok(value) = value.parse::<f64>() {
                row.numeric.insert(key.into(), value);
            } else if let Ok(value) = value.parse::<bool>() {
                row.flags.insert(key.into(), value);
            } else if value.starts_with('"') {
                row.strings
                    .insert(key.into(), serde_json::from_str(value).unwrap());
            }
        }
        assert!(current.is_none());
    }
    assert_eq!(assignments, 651);
    result
}
fn literal_curve(name: &str) -> Vec<f64> {
    let source = fs::read_to_string(root().join("src/Data/Misc.lua")).unwrap();
    let prefix = format!("data.{name} = {{ ");
    let body = source
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .unwrap()
        .strip_suffix(" }")
        .unwrap();
    body.split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.parse().unwrap())
        .collect()
}
#[test]
fn all_final_profiles_scalar_facts_and_ordered_children_match_literal_inventory() {
    let actual = export();
    let expected = literal_inventory();
    assert_eq!(actual.catalog().profiles.len(), 649);
    assert_eq!(expected.len(), 649);
    assert_eq!(actual.evidence().profiles, 649);
    assert_eq!(actual.evidence().child_skills, 2777);
    assert_eq!(actual.evidence().unconverted_modifiers, 661);
    assert_eq!(actual.evidence().numeric_field_names.len(), 21);
    assert_eq!(
        actual.evidence().boolean_field_names,
        ["baseDamageIgnoresAttackSpeed"]
    );
    assert!(
        actual
            .catalog()
            .profiles
            .windows(2)
            .all(|w| w[0].key < w[1].key)
    );
    for row in &actual.catalog().profiles {
        let expected = &expected[&row.key];
        assert_eq!(row.source_module, expected.module);
        assert_eq!(row.name, expected.strings["name"]);
        assert_eq!(row.child_skills, expected.skills, "{}", row.key);
        assert_eq!(
            row.unconverted_modifiers.as_ref().unwrap().len(),
            expected.modifiers,
            "{}",
            row.key
        );
        let mut numeric = BTreeMap::new();
        for (key, value) in [
            ("attackTime", row.attack_time),
            ("damage", row.damage_scale),
            ("damageSpread", row.damage_spread),
            ("critChance", row.critical_chance),
            ("attackRange", row.attack_range),
        ] {
            if let Some(value) = value {
                numeric.insert(key.to_owned(), value);
            }
        }
        let mut flags = BTreeMap::new();
        for (key, value) in [
            ("hostile", row.hostile),
            (
                "baseDamageIgnoresAttackSpeed",
                row.base_damage_ignores_attack_speed,
            ),
        ] {
            if let Some(value) = value {
                flags.insert(key.to_owned(), value);
            }
        }
        let mut strings = BTreeMap::from([("name".into(), row.name.clone())]);
        if let Some(value) = &row.weapon_family {
            strings.insert("weaponType1".into(), value.clone());
        }
        for (key, value) in &row.extra_facts {
            assert!(row.unconverted_fields.contains(key));
            match value {
                ActorScalarFact::Number(value) => {
                    numeric.insert(key.clone(), *value);
                }
                ActorScalarFact::Boolean(value) => {
                    flags.insert(key.clone(), *value);
                }
                ActorScalarFact::Text(value) => {
                    strings.insert(key.clone(), value.clone());
                }
            }
        }
        assert_eq!(numeric.len(), expected.numeric.len(), "{}", row.key);
        for (key, value) in &expected.numeric {
            assert_eq!(numeric[key].to_bits(), value.to_bits(), "{} {key}", row.key);
        }
        assert_eq!(flags, expected.flags, "{}", row.key);
        assert_eq!(strings, expected.strings, "{}", row.key);
    }
}
#[test]
fn summon_and_both_damage_curves_preserve_every_authored_value_without_selection() {
    let catalog = export().catalog();
    assert_eq!(catalog.summon_levels.first_level, 1);
    assert_eq!(catalog.summon_levels.rows.len(), 40);
    assert_eq!(
        catalog.summon_levels.rows,
        literal_curve("minionLevelTable")
            .into_iter()
            .map(|v| v as u32)
            .collect::<Vec<_>>()
    );
    for (curve, name) in [
        (&catalog.allied_damage, "monsterAllyDamageTable"),
        (&catalog.hostile_damage, "monsterDamageTable"),
    ] {
        assert_eq!(curve.first_level, 1);
        assert_eq!(curve.rows.len(), 100);
        assert_eq!(
            curve.rows.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            literal_curve(name)
                .into_iter()
                .map(f64::to_bits)
                .collect::<Vec<_>>()
        );
    }
    assert_ne!(catalog.allied_damage.rows, catalog.hostile_damage.rows);
}
#[test]
fn raw_zero_absence_overwrites_and_opaque_effects_stay_distinct() {
    let find = |key: &str| {
        export()
            .catalog()
            .profiles
            .iter()
            .find(|r| r.key == key)
            .unwrap()
    };
    let sniper = find("RaisedSkeletonSniper");
    assert_eq!(sniper.attack_time, Some(1.5));
    assert_eq!(sniper.damage_scale, Some(1.15));
    assert_eq!(sniper.damage_spread, Some(0.3));
    assert_eq!(sniper.weapon_family.as_deref(), Some("Bow"));
    assert_eq!(sniper.child_skills.len(), 2);
    assert_eq!(sniper.unconverted_modifiers, Some(vec![]));
    let djinn = find("SandDjinn");
    assert_eq!(djinn.attack_time, Some(0.0));
    assert_eq!(djinn.weapon_family, None);
    assert_eq!(djinn.child_skills[1], "ExplosiveTeleportSandDjinn");
    assert_eq!(djinn.unconverted_modifiers.as_ref().unwrap().len(), 3);
    assert_eq!(export().evidence().zero_attack_times, 8);
    assert_eq!(find("BeetleMinion").base_damage_ignores_attack_speed, None);
    assert_eq!(
        find("Metadata/Monsters/GoreCharger/GoreCharger").name,
        "Diretusk Boar"
    );
    let overwritten = find("Metadata/Monsters/GoreCharger/GoreCharger");
    assert_eq!(overwritten.damage_scale, Some(1.7));
    assert_eq!(overwritten.attack_time, Some(1.065));
    assert_eq!(
        overwritten.extra_facts["limit"],
        ActorScalarFact::Text("ActiveSpectreLimit".into())
    );
    assert_eq!(
        export()
            .catalog()
            .profiles
            .iter()
            .filter(|p| p.unconverted_flags.is_some())
            .count(),
        67
    );
    assert_eq!(
        export()
            .catalog()
            .profiles
            .iter()
            .filter_map(|p| p.unconverted_flags.as_ref())
            .map(BTreeMap::len)
            .sum::<usize>(),
        75
    );
    assert!(
        export()
            .catalog()
            .profiles
            .iter()
            .any(|p| p.extra_facts.contains_key("weaponType2"))
    );
}
#[test]
fn independent_source_and_exact_catalog_authentication_reject_content_spoofing() {
    let actual = export();
    assert_eq!(actual.catalog().source.files.len(), 4);
    assert_eq!(
        actual.evidence().source_manifest_sha256,
        source::manifest_sha256()
    );
    assert_eq!(
        actual.evidence().catalog_sha256,
        sha(actual.catalog_bytes())
    );
    for pin in &actual.catalog().source.files {
        assert_eq!(pin.sha256, source::expected_file_sha256(&pin.path).unwrap());
    }
    assert_eq!(actual.catalog_bytes().last(), Some(&b'\n'));
    let decoded: ActorBaselineCatalog = serde_json::from_slice(actual.catalog_bytes()).unwrap();
    actual.validate_catalog(&decoded).unwrap();
    for case in 0..6 {
        let mut forged = decoded.clone();
        match case {
            0 => forged.profiles[0].damage_scale = Some(0.0),
            1 => {
                forged.profiles.pop();
            }
            2 => forged
                .profiles
                .iter_mut()
                .find(|p| p.child_skills.len() > 1)
                .unwrap()
                .child_skills
                .swap(0, 1),
            3 => forged.allied_damage.rows[0] += 1.0,
            4 => {
                forged
                    .profiles
                    .iter_mut()
                    .find(|p| p.attack_time.is_some())
                    .unwrap()
                    .attack_time = Some(f64::NAN)
            }
            _ => {
                forged
                    .profiles
                    .iter_mut()
                    .find(|p| p.attack_time == Some(0.0))
                    .unwrap()
                    .attack_time = Some(-0.0)
            }
        }
        assert_eq!(forged.source, decoded.source);
        assert!(actual.validate_catalog(&forged).is_err(), "case {case}");
    }
    let temp = tempfile::tempdir().unwrap();
    assert!(export_owned_actor_baselines(temp.path(), Default::default()).is_err());
}
#[test]
fn caller_bounds_reject_oversized_source_inventory_and_publication() {
    for limits in [
        ActorBaselineExportLimits {
            max_source_bytes: 1,
            ..Default::default()
        },
        ActorBaselineExportLimits {
            max_profiles: 648,
            ..Default::default()
        },
        ActorBaselineExportLimits {
            max_catalog_bytes: 100,
            ..Default::default()
        },
        ActorBaselineExportLimits {
            max_total_entries: 1,
            ..Default::default()
        },
    ] {
        assert!(export_owned_actor_baselines(&root(), limits).is_err());
    }
    assert!(
        export_owned_actor_baselines(
            &root(),
            ActorBaselineExportLimits {
                max_profiles: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
}
