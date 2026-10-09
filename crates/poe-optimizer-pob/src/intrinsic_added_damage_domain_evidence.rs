//! Offline census of the two original added-damage scaling channels.
//! The original constructors execute unchanged; this is not a runtime evaluator.
use super::*;
#[path = "../../../tests/support/owned_combined_added_attack_damage_evidence.rs"]
mod evidence;
const ADDED_PACKET: &str =
    "data/owned/poe2/3887ae68/combined-added-attack-damage/source-evidence.json";
const KEYS: [&str; 2] = [
    "added_damage_+%_final",
    "active_skill_added_damage_+%_final",
];

fn capture_added(sources: &BTreeMap<String, String>) -> Result<Json> {
    let (lua, result, module_order) = construct_catalog(sources)?;
    let data: Table = result.get("data")?;
    let skills: Table = data.get("skills")?;
    let (mut inventory, minions) = catalog_inventory(&lua, sources, &data, &result)?;
    let base_tables = inventory.seen.len();
    let base_rows = inventory.rows;
    let base_aliases = inventory.aliases.clone();
    let cache: Table = lua
        .load(source(sources, "src/Data/ModCache.lua")?)
        .set_name("@src/Data/ModCache.lua")
        .eval()?;
    inventory.visit(sources, "modCache", Value::Table(cache), 0)?;
    let mut extra_stats = Vec::new();
    let mut modifiers = Vec::new();
    for (path, table) in &inventory.all_modifiers {
        let name = required_text(table, "name")?;
        if name == "ExtraSkillStat" {
            extra_stats.push(json!({"path":path,"record":plain(Value::Table(table.clone()),0)?}));
        }
        if matches!(name.as_str(), "AddedDamage" | "AddedPhysicalDamage") {
            modifiers.push(json!({"path":path,"record":plain(Value::Table(table.clone()),0)?}));
        }
    }
    let profile: Table = minions.raw_get(PROFILE)?;
    let selection = capture(sources)?;
    let mut references = Vec::new();
    for (path, text) in sources {
        if !(path.starts_with("src/Modules/")
            || path.starts_with("src/Classes/")
            || path.starts_with("src/Data/"))
        {
            continue;
        }
        for token in tokens(text)? {
            if token.quoted
                && (token.text.contains("Added") || token.text.contains("added_damage_+%_final"))
            {
                let value: String = lua.load(format!("return {}", token.text)).eval()?;
                if matches!(
                    value.as_str(),
                    "AddedDamage"
                        | "AddedPhysicalDamage"
                        | "Added"
                        | "added_damage_+%_final"
                        | "active_skill_added_damage_+%_final"
                ) {
                    references.push(json!({"path":path,"line":token.line,"literal":value}));
                }
            }
        }
    }
    // Evaluate only the exact original, all-literal precision table. Its full
    // file is pinned; no replacement data or arithmetic is introduced.
    let text = source(sources, DATA)?;
    let start = text
        .find("data.highPrecisionMods = {")
        .ok_or_else(|| error("precision table missing"))?
        + "data.highPrecisionMods = ".len();
    let end = start
        + text[start..]
            .find("\n}\n")
            .ok_or_else(|| error("precision table end missing"))?
        + 2;
    let precision: Table = lua
        .load(format!("return {}", &text[start..end]))
        .set_name("@src/Modules/Data.lua#highPrecisionMods")
        .eval()?;
    Ok(
        json!({"schema_version":1,"kind":"pinned-constructed-intrinsic-added-damage-domain",
        "upstream_revision":crate::source::UPSTREAM_REVISION,"source_manifest_sha256":crate::source::manifest_sha256(),
        "scope":{"normal_import_only":true,"imported_party_admitted":false,"requires_accounted_input":true,"exact_intrinsic_basic_source":true,"other_profiles_admitted":false,"all_actions":false,"finite_numeric_domain":[1.0,1.15],"game_rounding_intent":false},
        "files":selection["files"],"module_order":module_order,
        "catalog":{"skill_count":selection["catalog"]["skill_count"],"raw_tables":base_tables,"raw_rows":base_rows,
            "added_modifiers":modifiers,"stat_consumers":stat_consumers(&skills,&KEYS.iter().map(|s|s.to_string()).collect())?,
            "callbacks":inventory.callbacks,"aliases":base_aliases,"metatables":inventory.metatables,
            "selected":selection["catalog"]["selected"],"full_profile":plain(Value::Table(profile),0)?},
        "cache_census":{"raw_tables":inventory.seen.len(),"raw_rows":inventory.rows,"modifiers":inventory.modifier_count,"aliases":inventory.aliases},
        "extra_stats":extra_stats,"extra_stat_factories":coefficient::extra_stat_factories(&lua,sources)?,
        "references":references,"high_precision_mods":plain(Value::Table(precision),0)?,
        "selection_dependency":{"path":PACKET,"sha256":"a2daf1e5c6c058ae2ce32f434feee325d06344a96a5ccbf29ed7320df2c63ec3"},
        "selected_pass_corroboration":selection["selected_pass_corroboration"]}),
    )
}
#[test]
#[ignore = "authors bounded added-domain evidence using original constructors"]
fn author_added_damage_domain() {
    let actual = capture_added(sources()).unwrap();
    evidence::check(&actual, true);
    let path = root().join(ADDED_PACKET);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, serde_json::to_vec_pretty(&actual).unwrap()).unwrap();
}
#[test]
fn constructed_added_damage_domain_matches_retained_evidence() {
    let actual = capture_added(sources()).unwrap();
    evidence::check(&actual, true);
    let expected: Json =
        serde_json::from_slice(&std::fs::read(root().join(ADDED_PACKET)).unwrap()).unwrap();
    assert!(actual == expected, "constructed added-domain proof changed");
}
#[test]
fn real_constructed_writers_and_stat_consumers_refuse_new_domains_without_stale_hashes() {
    for body in [
        "baseMods={mod(\"AddedDamage\",\"INC\",0)},",
        "baseMods={mod(\"AddedDamage\",\"MORE\",0)},",
        "baseMods={mod(\"AddedPhysicalDamage\",\"MORE\",1)},",
        "baseMods={mod(\"Added\"..\"Damage\",\"MORE\",1)},",
        "baseMods={mod(\"AddedDamage\",\"MORE\",0,0,0,{type=\"Condition\",var=\"Unreviewed\"})},",
        "baseMods={mod(\"ExtraSkillMod\",\"LIST\",{mod=mod(\"AddedDamage\",\"MORE\",0)})},",
        "qualityStats={{\"added_damage_+%_final\",1}},",
        "qualityStats={{\"active_skill_added_damage_+%_final\",0}},",
    ] {
        let mut changed = sources().clone();
        let text = changed.get_mut("src/Data/Skills/minion.lua").unwrap();
        let needle = "skills[\"GasShotSkeletonSniperMinion\"] = {";
        assert!(text.contains(needle));
        let (field, value) = body.trim_end_matches(',').split_once('=').unwrap();
        *text = text.replacen(
            needle,
            &format!("skills[\"MinionMeleeBow\"].{field}={value}\n{needle}"),
            1,
        );
        let actual = capture_added(&changed).unwrap();
        if field == "baseMods" {
            assert!(
                actual["catalog"]["added_modifiers"]
                    .as_array()
                    .unwrap()
                    .len()
                    > 2,
                "actual constructed writer required"
            );
        } else {
            assert!(
                !actual["catalog"]["stat_consumers"]
                    .as_array()
                    .unwrap()
                    .is_empty(),
                "actual mapped stat consumer required"
            );
        }
        assert!(
            std::panic::catch_unwind(|| evidence::check_semantics(&actual)).is_err(),
            "accepted constructed {body}"
        );
    }
}
